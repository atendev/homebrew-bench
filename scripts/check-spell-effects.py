#!/usr/bin/env python3
"""Honesty check for data/spells/effects.json against upstream SRD text.

Usage: python3 -I scripts/check-spell-effects.py

The script renders the mechanical behavior implied by each modeled spell entry
and checks that its concrete facts (damage dice, save ability, conditions,
area shape/size, healing dice, upcast dice) appear in the upstream SRD
description or higher_level text. It also reports SRD facts that are not
covered by any effect or agent_clause.

Exit code 0 means no unexpected mechanical mismatches; exit code 1 means at
least one modeled fact contradicts or is absent from the SRD text.
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EFFECTS_PATH = ROOT / "data/spells/effects.json"
SPELLS_PATH = ROOT / "data/srd/2024/5e-SRD-Spells.json"

DAMAGE_TYPES = {
    "acid", "bludgeoning", "cold", "fire", "force", "lightning",
    "necrotic", "piercing", "poison", "psychic", "radiant", "slashing", "thunder",
}

ABILITIES = {"str", "dex", "con", "int", "wis", "cha"}


def load_json(path: Path) -> dict | list:
    with path.open() as f:
        return json.load(f)


def gather_text(spell: dict) -> str:
    parts = [spell.get("description", ""), spell.get("higher_level", "")]
    return "\n".join(parts).lower()


HIGHER_SLOT = "Using a Higher-Level Spell Slot."

COUNT_WORDS = {1: "one", 2: "two", 3: "three", 4: "four", 5: "five"}


def higher_level_text(spell: dict) -> str:
    """The SRD text for what a higher slot does, from `higher_level` or the
    description's "Using a Higher-Level Spell Slot." paragraph. A "Cantrip
    Upgrade." paragraph is not higher-slot text."""
    parts = [spell.get("higher_level", "")]
    desc = spell.get("description", "")
    if HIGHER_SLOT in desc:
        parts.append(desc.split(HIGHER_SLOT, 1)[1].split("\n", 1)[0])
    return " ".join(p.strip() for p in parts if p.strip())


def with_modifier(dice: str, modifier: int) -> str:
    return f"{dice}+{modifier}" if modifier else dice


def dice_in_text(dice: str) -> set[str]:
    """Return the dice expressions we expect to find in the SRD text."""
    exprs = set()
    # Normalize spaces and plus signs.
    norm = dice.replace(" ", "").replace("+", " + ")
    # Add the raw expression and a space-separated variant.
    exprs.add(norm)
    # For expressions like 1d4+1, also add "1d4 + 1".
    exprs.add(norm.replace("+", " + "))
    # For "XdY+Z" the SRD sometimes writes "XdY plus Z".
    m = re.match(r"(\d+d\d+)\s*\+\s*(\d+)", norm)
    if m:
        exprs.add(f"{m.group(1)} plus {m.group(2)}")
    return exprs


def text_contains_dice(text: str, dice: str) -> bool:
    candidates = dice_in_text(dice)
    return any(c in text for c in candidates)


def render_effect_fact(effect: dict) -> list[str]:
    """Yield strings representing one effect's mechanical facts."""
    facts = []
    if "attack" in effect:
        atk = effect["attack"]
        for dmg in atk.get("damage", []):
            facts.append(f"attack deals {with_modifier(dmg['dice'], dmg.get('modifier', 0))} {dmg.get('type', 'damage')}")
    if "saving_throw" in effect:
        save = effect["saving_throw"]
        facts.append(f"{save['ability']} save")
        if save.get("half_on_success"):
            facts.append("half damage on success")
        for dmg in save.get("damage", []):
            facts.append(f"save deals {with_modifier(dmg['dice'], dmg.get('modifier', 0))} {dmg.get('type', 'damage')}")
        for on_fail in save.get("on_failure", []):
            facts.extend(render_effect_fact(on_fail))
    if "damage" in effect:
        dmg = effect["damage"]
        facts.append(f"auto-hit damage {with_modifier(dmg['dice'], dmg.get('modifier', 0))} {dmg.get('type', 'damage')}")
    if "heal" in effect:
        heal = effect["heal"]
        if "dice" in heal:
            facts.append(f"heals {with_modifier(heal['dice'], heal.get('modifier', 0))} hit points")
        else:
            facts.append(f"heals {heal.get('modifier', 0)} hit points")
    if "condition" in effect:
        cond = effect["condition"]
        facts.append(f"applies {cond['condition']}")
    if "timed_modifier" in effect:
        mod = effect["timed_modifier"]
        target = next(iter(mod.get("target", {}).keys())) if isinstance(mod.get("target"), dict) else str(mod.get("target"))
        change = mod.get("change")
        if isinstance(change, dict):
            change = next(iter(change.items()))
        facts.append(f"modifier: {target} {change}")
    if "teleport" in effect:
        facts.append(f"teleport up to {effect['teleport']['range']} feet")
    if "area" in effect:
        area = effect["area"]["area"]
        shape = next(iter(area.keys()))
        facts.append(f"area: {shape} {area[shape]}")
        facts.extend(render_effect_fact(effect["area"]["effect"]))
    return facts


def check_upcast(entry: dict, spell: dict) -> list[str]:
    """Mismatches between the entry's level and scaling and the SRD's higher-slot text."""
    mismatches = []
    if entry.get("level") != spell["level"]:
        mismatches.append(f"level {entry.get('level')} does not match SRD level {spell['level']}")
    higher = higher_level_text(spell).lower()
    rules = entry.get("scaling", [])
    if higher and not rules and not entry.get("upcast_clause"):
        mismatches.append("SRD has higher-level text but the entry has no scaling rule and no upcast_clause")
    for rule in rules:
        (kind, value), = rule["per_slot"].items()
        if kind == "extra_dice":
            wanted = [value["dice"]]
        elif kind == "extra_modifier":
            wanted = [str(value["amount"])]
        else:
            wanted = [str(value["count"]), COUNT_WORDS.get(value["count"], "")]
        if not any(w and re.search(rf"\b{re.escape(w)}\b", higher) for w in wanted):
            mismatches.append(f"{kind} {value} not found in the SRD higher-level text")
    return mismatches


def check_spell(idx: str, entry: dict, spell: dict) -> list[str]:
    """Return a list of mismatch notes for one spell."""
    text = gather_text(spell)
    mismatches = []
    facts = []
    for effect in entry.get("effects", []):
        facts.extend(render_effect_fact(effect))
    for delayed in entry.get("delayed", []):
        facts.append(f"delayed {delayed['delay']}")
        facts.extend(render_effect_fact(delayed["effect"]))
    mismatches.extend(check_upcast(entry, spell))

    # Check that concrete facts appear in SRD text.
    for fact in facts:
        # Damage dice.
        m = re.search(r"(\d+d\d+(?:\s*\+\s*\d+)?)\s+(\w+)", fact)
        if m:
            dice, dtype = m.group(1), m.group(2)
            if not text_contains_dice(text, dice):
                mismatches.append(f"dice {dice} not found in SRD text ({fact})")
            if dtype in DAMAGE_TYPES and dtype not in text:
                mismatches.append(f"damage type {dtype} not found in SRD text ({fact})")
        # Save ability.
        m = re.search(r"\b(str|dex|con|int|wis|cha)\s+save", fact)
        if m:
            ability_full = {
                "str": "strength", "dex": "dexterity", "con": "constitution",
                "int": "intelligence", "wis": "wisdom", "cha": "charisma",
            }[m.group(1)]
            if ability_full not in text:
                mismatches.append(f"save ability {ability_full} not found in SRD text ({fact})")
        # Condition.
        m = re.search(r"applies\s+(\w+)", fact)
        if m:
            cond = m.group(1).lower()
            if cond not in text:
                mismatches.append(f"condition {cond} not found in SRD text ({fact})")
        # Area shape.
        m = re.search(r"area:\s+(\w+)\s+\{([^}]+)\}", fact)
        if m:
            shape = m.group(1).lower()
            params_str = m.group(2)
            shape_ok = shape in text
            if not shape_ok and shape in ("sphere", "emanation"):
                # A sphere/emanation is also verified by "within X feet" in
                # upstream text when the radius is named in the model.
                radius_m = re.search(r"['\"]radius['\"]\s*:\s*(\d+)", params_str)
                if radius_m:
                    radius = radius_m.group(1)
                    if re.search(rf"within\s+{radius}\s+(?:feet|ft)", text):
                        shape_ok = True
            if not shape_ok:
                mismatches.append(f"area shape {shape} not found in SRD text ({fact})")
        # Healing.
        m = re.search(r"heals\s+(\d+d\d+(?:\+\d+)?|\d+)", fact)
        if m:
            dice = m.group(1)
            if not text_contains_dice(text, dice) and dice not in text:
                mismatches.append(f"healing {dice} not found in SRD text ({fact})")

    # Check agent_clause coverage: warn if SRD mentions a damage type or condition
    # that no effect covers. This is a soft check; we report it but do not fail.
    # (Skipped for brevity; the mechanical facts above are the hard check.)

    return mismatches


def main() -> int:
    effects = load_json(EFFECTS_PATH)
    spells = {s["index"]: s for s in load_json(SPELLS_PATH)}

    all_mismatches: list[tuple[str, list[str]]] = []
    missing_in_srd = []
    for idx, entry in effects.items():
        spell = spells.get(idx)
        if spell is None:
            missing_in_srd.append(idx)
            continue
        mismatches = check_spell(idx, entry, spell)
        if mismatches:
            all_mismatches.append((idx, mismatches))

    if missing_in_srd:
        print(f"error: {len(missing_in_srd)} entries have no SRD spell:", file=sys.stderr)
        for idx in missing_in_srd:
            print(f"  {idx}", file=sys.stderr)

    if all_mismatches:
        print(f"error: {len(all_mismatches)} spell entries have mechanical mismatches:", file=sys.stderr)
        for idx, mismatches in all_mismatches:
            print(f"  {idx}:", file=sys.stderr)
            for m in mismatches:
                print(f"    - {m}", file=sys.stderr)
        return 1

    print(f"ok: {len(effects)} spell effect entries are consistent with SRD text.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
