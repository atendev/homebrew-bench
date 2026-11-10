#!/usr/bin/env python3
"""Scaffold starter entries for data/spells/effects.json from SRD spell data.

Usage:
  python3 -I scripts/scaffold-spells.py IDs... > data/spells/effects.json
  python3 -I scripts/scaffold-spells.py --party > data/spells/effects.json
  python3 -I scripts/scaffold-spells.py --monster-core [TOP_N] > data/spells/effects.json

The script reads data/srd/2024/5e-SRD-Spells.json and emits a JSON object keyed by
SRD spell index. Each value is a starter SpellEffect entry using the closed effect
vocabulary defined in bench-core. Hand-finishing is required before the entry is
considered modeled.

The output is intentionally conservative: anything the structured fields cannot
map cleanly is left as an agent_clause or note.
"""

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPELLS_PATH = ROOT / "data/srd/2024/5e-SRD-Spells.json"
PARTY_SPELLS_PATH = ROOT / "data/party/standard-spells.json"
SOURCE = "SRD 5.2.1"

DAMAGE_TYPE_MAP = {
    "acid": "acid",
    "bludgeoning": "bludgeoning",
    "cold": "cold",
    "fire": "fire",
    "force": "force",
    "lightning": "lightning",
    "necrotic": "necrotic",
    "piercing": "piercing",
    "poison": "poison",
    "psychic": "psychic",
    "radiant": "radiant",
    "slashing": "slashing",
    "thunder": "thunder",
}

CONDITIONS = [
    "Blinded",
    "Charmed",
    "Deafened",
    "Exhaustion",
    "Frightened",
    "Grappled",
    "Incapacitated",
    "Invisible",
    "Paralyzed",
    "Petrified",
    "Poisoned",
    "Prone",
    "Restrained",
    "Stunned",
    "Unconscious",
]

ABILITY_RE = re.compile(
    r"\b(Strength|Dexterity|Constitution|Intelligence|Wisdom|Charisma)\s+saving\s+throw",
    re.IGNORECASE,
)
RANGE_RE = re.compile(r"^(\d+)\s+feet", re.IGNORECASE)
SHAPE_RE = re.compile(
    r"(\d+)-\s?foot-?(?:radius\s*,?\s*)?(?:(\d+)-\s?foot-?)?(?:high\s*)?(Cone|Sphere|Cube|Emanation|Cylinder)",
    re.IGNORECASE,
)
LINE_RE = re.compile(
    r"(\d+)-\s?foot-?(?:long,?\s*)?(?:and\s*(\d+)-\s?foot-?\s*wide,?\s*)?Line",
    re.IGNORECASE,
)
AREA_ORIGIN_RE = re.compile(
    r"(originating from you|centered on you|centered on a point you choose|centered on that point|centered there|within range)",
    re.IGNORECASE,
)


def normalize_range(text: str) -> dict:
    """Return a dict with kind and distance where possible."""
    text = text.strip()
    if text.lower().startswith("self"):
        return {"kind": "self"}
    if text.lower().startswith("touch"):
        return {"kind": "touch"}
    m = RANGE_RE.match(text)
    if m:
        return {"kind": "feet", "distance": int(m.group(1))}
    if "unlimited" in text.lower() or "sight" in text.lower() or "mile" in text.lower():
        return {"kind": "special", "text": text}
    return {"kind": "unknown", "text": text}


def normalize_duration(text: str, concentration: bool) -> dict:
    text = text.strip()
    if text.lower().startswith("instant"):
        return {"kind": "instant"}
    if text.lower().startswith("1 round"):
        return {"kind": "rounds", "value": 1}
    if text.lower().startswith("up to 1 minute"):
        return {"kind": "minutes", "value": 1, "concentration": concentration}
    if text.lower().startswith("up to 10 minutes"):
        return {"kind": "minutes", "value": 10, "concentration": concentration}
    if text.lower().startswith("up to 1 hour"):
        return {"kind": "hours", "value": 1, "concentration": concentration}
    if text.lower().startswith("up to 8 hours"):
        return {"kind": "hours", "value": 8, "concentration": concentration}
    if text.lower().startswith("8 hours"):
        return {"kind": "hours", "value": 8}
    if text.lower().startswith("24 hours"):
        return {"kind": "hours", "value": 24}
    if text.lower().startswith("1 hour"):
        return {"kind": "hours", "value": 1, "concentration": concentration}
    if text.lower().startswith("1 minute"):
        return {"kind": "minutes", "value": 1, "concentration": concentration}
    if text.lower().startswith("10 minutes"):
        return {"kind": "minutes", "value": 10, "concentration": concentration}
    if text.lower().startswith("1 day") or text.lower().startswith("up to 1 day"):
        return {"kind": "days", "value": 1}
    if "until dispelled" in text.lower():
        return {"kind": "permanent"}
    return {"kind": "unknown", "text": text, "concentration": concentration}


def make_duration(dur: dict) -> dict:
    if dur["kind"] == "instant":
        return {"instant": None}
    if dur["kind"] == "rounds":
        return {"rounds": dur["value"]}
    if dur["kind"] == "minutes":
        return {"minutes": dur["value"]}
    if dur["kind"] == "hours":
        return {"hours": dur["value"]}
    if dur["kind"] == "days":
        return {"days": dur["value"]}
    if dur["kind"] == "permanent":
        return {"permanent": None}
    return {"rounds": 1}


def damage_type(srd_type: str) -> str:
    return DAMAGE_TYPE_MAP.get(srd_type.lower(), srd_type.lower())


def split_dice(expr: str) -> tuple[str | None, int]:
    """Split "10d6+40" into its dice and flat part; a bare number has no dice."""
    expr = expr.replace(" ", "")
    m = re.fullmatch(r"(\d+d\d+)?(?:\+?(\d+))?", expr)
    if not m:
        return expr, 0
    return m.group(1), int(m.group(2) or 0)


def damage_entry(expr: str, dtype: str) -> dict:
    dice, modifier = split_dice(expr)
    entry = {"dice": dice, "type": dtype}
    if modifier:
        entry["modifier"] = modifier
    return entry


def parse_dice_from_desc(desc: str, damage_type_hint: str | None) -> dict | None:
    """Best-effort extraction of a damage expression from prose."""
    # Look for "XdY [type] damage"
    if damage_type_hint:
        rx = re.compile(
            rf"(\d+d\d+(?:\s*\+\s*\d+)?)\s+{re.escape(damage_type_hint)}\s+damage",
            re.IGNORECASE,
        )
        m = rx.search(desc)
        if m:
            return damage_entry(m.group(1), damage_type(damage_type_hint))
    # Generic: take first "XdY ... damage" with a known type.
    for m in re.finditer(r"(\d+d\d+(?:\s*\+\s*\d+)?)\s+([A-Za-z]+)\s+damage", desc, re.IGNORECASE):
        t = damage_type(m.group(2))
        if t in DAMAGE_TYPE_MAP:
            return damage_entry(m.group(1), t)
    return None


def infer_save_ability(desc: str) -> str | None:
    for m in ABILITY_RE.finditer(desc):
        before = desc[: m.start()].rstrip()
        # Skip if the save is only mentioned as a modifier target (advantage, bonus, penalty).
        if re.search(r"\b(?:Advantage|Disadvantage|bonus|penalty|adds?)\s+(?:[^,.;]{0,40})$", before, re.IGNORECASE):
            continue
        # Skip if the save is preceded by "saving throws" (the phrase we matched is a repeat target).
        if re.search(r"\bsaving\s+throws?\b[^,.;]{0,40}$", before, re.IGNORECASE):
            continue
        return m.group(1).lower()[:3]
    return None


def infer_half_damage(desc: str) -> bool:
    return "half as much damage" in desc.lower() or "half damage" in desc.lower()


def infer_no_damage_on_success(desc: str) -> bool:
    d = desc.lower()
    # Common phrasing for nothing-on-success.
    return "or take no damage" in d or "takes nothing" in d or "no damage on a successful" in d


def infer_conditions(desc: str) -> list[str]:
    found = []
    for c in CONDITIONS:
        # "has the Charmed condition", "is Charmed" — but avoid phrases like
        # "When the spell ends, the target is Incapacitated ..." which describe ending,
        # or "ends the Blinded, Deafened, and Poisoned conditions" which describe removal.
        for m in re.finditer(
            rf"\b(?:has|have|gains?|becomes?|is|are)\s+(?:the\s+)?{c}\b(?:\s+condition)?",
            desc,
            re.IGNORECASE,
        ):
            before = desc[: m.start()].rstrip()
            after = desc[m.end() : m.end() + 80]
            # Skip if preceded by a clear end-of-spell phrase within the same sentence.
            if re.search(r"\b(?:When the spell ends|After the spell ends|At the end of this spell|once the spell ends)\b[^,.;]*$", before, re.IGNORECASE):
                continue
            # Skip if preceded by removal words in the same clause.
            if re.search(r"\b(?:ends?|removes?|cures?|is no longer)\b[^,.;]*$", before, re.IGNORECASE):
                continue
            # Skip if the phrase describes the target's own turn ending (Haste lethargy).
            if re.search(r"\buntil the end of (?:its|your) next turn\b.*\b(?:wave of lethargy|Speed of 0)\b", after, re.IGNORECASE):
                continue
            found.append(c.lower().replace(" ", "_"))
            break
    return found


def infer_area(desc: str) -> dict | None:
    m = SHAPE_RE.search(desc)
    if m:
        size = int(m.group(1))
        height = m.group(2)
        shape = m.group(3).lower()
        area = {"shape": shape}
        if shape == "cone":
            area["size"] = size
        elif shape == "line":
            area["length"] = size
            area["width"] = int(height) if height else 5
        elif shape == "sphere":
            area["radius"] = size
        elif shape == "cube":
            area["size"] = size
        elif shape == "emanation":
            area["radius"] = size
        elif shape == "cylinder":
            area["radius"] = size
            area["height"] = int(height) if height else 0
        return area
    m = LINE_RE.search(desc)
    if m:
        return {"shape": "line", "length": int(m.group(1)), "width": int(m.group(2)) if m.group(2) else 5}
    return None


def infer_origin(desc: str) -> str:
    d = desc.lower()
    if "originating from you" in d or "centered on you" in d or "blasts out from you" in d or "shoots forth from you" in d:
        return "self_"
    if "centered on a point" in d or "within range" in d:
        return "target_point"
    return "target_point"


HIGHER_SLOT = "Using a Higher-Level Spell Slot."


def higher_level_text(spell: dict) -> str:
    """What a higher slot does, from `higher_level` or the description's
    "Using a Higher-Level Spell Slot." paragraph. "Cantrip Upgrade." is not
    higher-slot text."""
    parts = [spell.get("higher_level", "")]
    desc = spell.get("description", "")
    if HIGHER_SLOT in desc:
        parts.append(desc.split(HIGHER_SLOT, 1)[1].split("\n", 1)[0])
    return " ".join(p.strip() for p in parts if p.strip())


def parse_upcast(higher: str) -> tuple[list[dict], str | None]:
    """Scaling rules for the four patterns the engine applies, and an
    `upcast_clause` holding the text when anything is left over."""
    text = higher.lower()
    rules = []
    increases = re.findall(r"increases by (\d+d\d+|\d+)\b", text)
    if increases:
        amount = increases[0]
        if "d" in amount:
            rule = {"dice": amount}
            m = re.search(r"(\w+) damage[^.]*?increases by " + re.escape(amount), text)
            if m and m.group(1) in DAMAGE_TYPE_MAP:
                rule["damage_type"] = m.group(1)
            rules.append({"per_slot": {"extra_dice": rule}})
        else:
            rules.append({"per_slot": {"extra_modifier": {"amount": int(amount)}}})
    elif re.search(r"one (?:more|additional) (?:dart|ray|beam)", text):
        rules.append({"per_slot": {"extra_effects": {"count": 1}}})
    elif re.search(r"one additional (?:\w+ )?(?:target|creature|bolt|humanoid)", text):
        rules.append({"per_slot": {"extra_targets": {"count": 1}}})
    clause = None if rules and len(increases) <= 1 else higher
    return rules, clause


def load_spells() -> dict[str, dict]:
    with SPELLS_PATH.open() as f:
        spells = json.load(f)
    return {s["index"]: s for s in spells}


def load_party_spell_ids() -> set[str]:
    with PARTY_SPELLS_PATH.open() as f:
        data = json.load(f)
    ids: set[str] = set()
    for role in ("caster", "healer"):
        ids.update(s["id"] for s in data[role].get("at_will", []))
        for tier in data[role].get("by_tier", {}).values():
            for entry in tier:
                ids.add(entry[0])
    return ids


def load_monster_core_spell_ids(top_n: int | None = None) -> list[str]:
    import collections

    monsters = json.loads((ROOT / "data/srd/2024/5e-SRD-Monsters.json").read_text())
    spells = load_spells()
    spell_by_name = {s["name"].lower(): idx for idx, s in spells.items()}
    counts = collections.Counter()
    for m in monsters:
        for section in ("special_abilities", "actions", "bonus_actions", "reactions", "legendary_actions"):
            for a in m.get(section, []):
                text = a.get("desc", "").lower()
                for name, idx in spell_by_name.items():
                    if re.search(rf"\b{re.escape(name)}\b", text):
                        counts[idx] += 1
    ordered = [idx for idx, _ in counts.most_common()]
    if top_n is not None:
        ordered = ordered[:top_n]
    return ordered


def scaffold(spell: dict) -> dict:
    idx = spell["index"]
    name = spell["name"]
    level = spell["level"]
    desc = spell.get("description", "")
    higher = higher_level_text(spell)
    concentration = spell.get("concentration", False)

    dur = normalize_duration(spell.get("duration", ""), concentration)
    rng = normalize_range(spell.get("range", ""))

    effects: list[dict] = []
    agent_clauses: list[str] = []
    scaling: list[dict] = []
    delayed: list[dict] = []
    notes: list[str] = []

    srd_damage = spell.get("damage", {})
    srd_damage_type = srd_damage.get("damage_type", {}).get("index") if srd_damage else None
    srd_dice_by_level = srd_damage.get("damage_at_slot_level", {}) if srd_damage else {}
    base_dice = srd_dice_by_level.get(str(level))

    save_ability = infer_save_ability(desc)
    half = infer_half_damage(desc)
    no_damage = infer_no_damage_on_success(desc)
    conditions = infer_conditions(desc)
    area = infer_area(desc)
    attack_type = spell.get("attack_type")
    origin = infer_origin(desc)

    if base_dice and srd_damage_type:
        # Web's damage_at_slot_level is the secondary fire damage; ignore it for the primary effect.
        if idx == "web":
            base_dice = None
            area = {"shape": "cube", "size": 20}
            effects.append({"area": {
                "area": make_area(area),
                "origin": "target_point",
                "effect": {"saving_throw": {
                    "ability": "dex",
                    "dc": 0,
                    "half_on_success": False,
                    "no_damage_on_success": False,
                    "damage": [],
                    "on_failure": [{"condition": {"condition": "restrained", "duration": {"concentration": None}, "source_ability": idx}}],
                }},
            }})
            agent_clauses.append("Webs are Difficult Terrain and Lightly Obscured; fire destroys a 5-foot Cube and deals 2d4 Fire damage to creatures starting their turn in it.")
        else:
            dmg = {**damage_entry(base_dice, damage_type(srd_damage_type)), "magical": True}

            if attack_type:
                # Spell attack roll.
                attack = {
                    "modifier": 0,
                    "target_count": 1,
                    "damage": [dmg],
                }
                if rng["kind"] == "feet":
                    attack["range"] = (rng["distance"], rng["distance"])
                effects.append({"attack": attack})
            elif save_ability:
                save = {
                    "ability": save_ability,
                    "dc": 0,
                    "half_on_success": half,
                    "no_damage_on_success": no_damage,
                    "damage": [dmg],
                }
                if conditions:
                    save["on_failure"] = [{"condition": {"condition": conditions[0], "duration": make_duration(dur), "source_ability": idx}}]
                effects.append({"saving_throw": save})
            elif area:
                # Area damage without a save is unusual; wrap as area damage.
                area_effect = {
                    "area": make_area(area),
                    "origin": origin,
                    "effect": {"damage": dmg},
                }
                effects.append({"area": area_effect})
            else:
                effects.append({"damage": dmg})
    elif save_ability:
        save = {
            "ability": save_ability,
            "dc": 0,
            "half_on_success": half,
            "no_damage_on_success": no_damage,
            "damage": [],
        }
        if conditions:
            save["on_failure"] = [{"condition": {"condition": conditions[0], "duration": make_duration(dur), "source_ability": idx}}]
        if area:
            area_effect = {
                "area": make_area(area),
                "origin": origin,
                "effect": {"saving_throw": save},
            }
            effects.append({"area": area_effect})
        else:
            effects.append({"saving_throw": save})
    elif attack_type:
        # Spell attack without structured damage (e.g. Spiritual Weapon, Arcane Sword).
        dmg = parse_dice_from_desc(desc, None)
        if dmg:
            attack = {
                "modifier": 0,
                "target_count": 1,
                "damage": [{**dmg, "magical": True}],
            }
            if rng["kind"] == "feet":
                attack["range"] = (rng["distance"], rng["distance"])
            effects.append({"attack": attack})
        else:
            agent_clauses.append(f"Make a {attack_type} spell attack; damage is not expressed in a closed effect.")
    elif conditions and not area:
        # Condition-only save-or-suck (rare without save_ability).
        for c in conditions:
            effects.append({"condition": {"condition": c, "duration": make_duration(dur), "source_ability": idx}})
    elif conditions and area:
        # Area condition without save detection is odd; treat as agent clause.
        agent_clauses.append(f"Area applies the following conditions: {', '.join(conditions)}.")

    # Detect healing.
    heal_expr = None
    # 1. "regains [a number of] Hit Points equal to X"
    m = re.search(r"regains?\s+(?:a number of\s+)?Hit Points?\s+equal to\s+([^,.;]+)", desc, re.IGNORECASE)
    if m:
        heal_expr = m.group(1).strip()
    # 2. "restores/restoring X Hit Points"
    if heal_expr is None:
        m = re.search(r"restor(?:e[ds]?|ing)\s+([^,.;]{1,60}?)\s+Hit Points?", desc, re.IGNORECASE)
        if m:
            heal_expr = m.group(1).strip()
    # 3. "regains X Hit Points"
    if heal_expr is None:
        m = re.search(r"regains?\s+(\d+|[\d\s+d+-]+)\s*Hit Points?", desc, re.IGNORECASE)
        if m:
            heal_expr = m.group(1).strip()

    if heal_expr:
        dice_m = re.search(r"(\d+d\d+(?:\s*\+\s*\d+)?)", heal_expr)
        mod_m = re.search(r"(?:plus|\+)\s*your\s*spellcasting\s*ability\s*modifier", heal_expr, re.IGNORECASE)
        if dice_m:
            dice, modifier = split_dice(dice_m.group(1))
            heal = {"dice": dice, "modifier": modifier}
            if not mod_m:
                heal["spellcasting_modifier"] = False
            effects.append({"heal": heal})
        elif re.search(r"all (?:its|their) Hit Points", desc, re.IGNORECASE):
            agent_clauses.append("Restores all Hit Points; no closed effect for full heal.")
        else:
            num_m = re.search(r"(\d+)", heal_expr)
            if num_m:
                effects.append({"heal": {"modifier": int(num_m.group(1)), "spellcasting_modifier": False}})

    # Special: mass heal divides 700 HP among targets and removes conditions.
    if idx == "mass-heal":
        agent_clauses.append("Distributes up to 700 Hit Points among any number of visible targets; no closed effect for split healing or condition removal.")

    # Detect temp HP.
    temphp_match = re.search(r"gains?\s+(?:a number of\s+)?Temporary\s+Hit\s+Points?\s+equal to\s+([^,.;]+)", desc, re.IGNORECASE)
    if temphp_match:
        expr = temphp_match.group(1).strip()
        dice_m = re.search(r"(\d+d\d+(?:\s*\+\s*\d+)?)", expr)
        if dice_m:
            effects.append({"temp_hp": {"dice": dice_m.group(1).replace(" ", "")}})

    # Detect teleports.
    teleport_match = re.search(r"teleport\s+(up to\s+)?(\d+)\s+feet", desc, re.IGNORECASE)
    if teleport_match:
        effects.append({"teleport": {"range": int(teleport_match.group(2)), "destination": "unoccupied space you can see"}})

    # Detect modifiers.
    if re.search(r"\+\d+\s+bonus to\s+(?:Armor Class|AC)", desc, re.IGNORECASE):
        m = re.search(r"\+(\d+)\s+bonus to\s+(?:Armor Class|AC)", desc, re.IGNORECASE)
        effects.append({"timed_modifier": {
            "target": {"armor_class": None},
            "change": {"add": int(m.group(1))},
            "duration": make_duration(dur),
            "source_ability": idx,
        }})
    if re.search(r"Advantage on\s+(Dexterity\s+)?saving throws", desc, re.IGNORECASE):
        ability = None
        if "Dexterity saving throws" in desc:
            ability = "dex"
        effects.append({"timed_modifier": {
            "target": {"saving_throws": {"ability": ability}},
            "change": "advantage",
            "duration": make_duration(dur),
            "source_ability": idx,
        }})
    if re.search(r"Advantage on\s+attack rolls", desc, re.IGNORECASE):
        effects.append({"timed_modifier": {
            "target": {"attack_rolls": None},
            "change": "advantage",
            "duration": make_duration(dur),
            "source_ability": idx,
        }})
    # Bless and Bane: a die added to or subtracted from attack rolls and saves; under a failed
    # save's on_failure when the text says only targets that fail it are changed.
    for verb, key in ((r"adds?", "add_dice"), (r"subtracts?", "subtract_dice")):
        m = re.search(rf"\b{verb}\s+(\d+d\d+)\s+(?:to|from)\s+the attack roll or (?:the\s+)?save", desc, re.IGNORECASE)
        if not m:
            continue
        dice_mods = [{"timed_modifier": {
            "target": target,
            "change": {key: m.group(1)},
            "duration": {"concentration": None} if concentration else make_duration(dur),
            "source_ability": idx,
        }} for target in ({"attack_rolls": None}, {"saving_throws": {"ability": None}})]
        saves = [e["saving_throw"] for e in effects if "saving_throw" in e]
        if saves and re.search(r"fails this save", desc, re.IGNORECASE):
            saves[0].setdefault("on_failure", []).extend(dice_mods)
        else:
            effects.extend(dice_mods)
    if re.search(r"Speed is doubled", desc, re.IGNORECASE):
        effects.append({"timed_modifier": {
            "target": {"speed": None},
            "change": "double",
            "duration": make_duration(dur),
            "source_ability": idx,
        }})
    if re.search(r"Speed is halved", desc, re.IGNORECASE):
        effects.append({"timed_modifier": {
            "target": {"speed": None},
            "change": "halve",
            "duration": make_duration(dur),
            "source_ability": idx,
        }})
    if re.search(r"can't take Reactions", desc, re.IGNORECASE):
        effects.append({"timed_modifier": {
            "target": {"reactions": None},
            "change": "none",
            "duration": make_duration(dur),
            "source_ability": idx,
        }})
    if re.search(r"a?\s*[-+]\d+\s+penalty to\s+(?:AC|Armor Class)", desc, re.IGNORECASE):
        m = re.search(r"a?\s*(-\d+)\s+penalty to\s+(?:AC|Armor Class)", desc, re.IGNORECASE)
        if m:
            effects.append({"timed_modifier": {
                "target": {"armor_class": None},
                "change": {"add": int(m.group(1))},
                "duration": make_duration(dur),
                "source_ability": idx,
            }})
    dex_penalty = re.search(r"[-+]\d+\s+penalty to\s+(?:Dexterity\s+)?saving throws", desc, re.IGNORECASE)
    if dex_penalty:
        m = re.search(r"(-\d+)\s+penalty to", desc, re.IGNORECASE)
        effects.append({"timed_modifier": {
            "target": {"saving_throws": {"ability": "dex"}},
            "change": {"add": int(m.group(1))},
            "duration": make_duration(dur),
            "source_ability": idx,
        }})

    # Magic Missile: auto-hitting force darts. Represent as three damage instances;
    # hand-finishing may convert to a multi-target effect when the vocabulary supports it.
    if idx == "magic-missile":
        effects = [{"damage": {"dice": "1d4", "modifier": 1, "type": "force", "magical": True}} for _ in range(3)]
        agent_clauses.append("Each dart hits a visible target of the caster's choice; the engine treats them as auto-hit damage instances.")

    # Scorching Ray: three attack rolls.
    if idx == "scorching-ray":
        effects = [{"attack": {"modifier": 0, "target_count": 1, "range": [120, 120], "damage": [{"dice": "2d6", "type": "fire", "magical": True}]}} for _ in range(3)]
        agent_clauses.append("Distribute rays among targets as desired.")

    # Acid Arrow: delayed secondary damage.
    if idx == "acid-arrow":
        delayed = [{"delay": {"rounds": 1}, "effect": {"damage": {"dice": "2d4", "type": "acid", "magical": True}}}]

    upcast_clause = None

    # Higher-slot text: rules on the first effect, the rest as an upcast_clause.
    if higher:
        scaling, upcast_clause = parse_upcast(higher)
        if not effects:
            scaling, upcast_clause = [], higher

    if concentration and dur["kind"] != "instant":
        # Ensure any timed modifier/condition duration reflects concentration if it was unknown.
        for eff in effects:
            for key in ("condition", "timed_modifier"):
                if key in eff:
                    if "duration" in eff[key] and eff[key]["duration"] != {"concentration": None}:
                        # Only switch to concentration if the scaffold mapped a bounded duration.
                        if isinstance(eff[key]["duration"], dict) and "minutes" in eff[key]["duration"]:
                            eff[key]["duration"] = {"concentration": None}

    if rng["kind"] == "self":
        for eff in effects:
            if "timed_modifier" in eff:
                eff["timed_modifier"]["bearer"] = "self"

    # Fallback: if we produced no effects, this entry is unmodeled.
    if not effects:
        agent_clauses.append("No closed effect mapped from SRD fields; resolve as agent judgment.")

    entry: dict = {
        "id": idx,
        "name": name,
        "source": SOURCE,
        "level": level,
        "concentration": concentration,
        "effects": effects,
    }
    if scaling:
        # Attach effect_index=0 by default; hand-finishers must correct multi-effect spells.
        entry["scaling"] = [{"effect_index": 0, **rule} for rule in scaling]
    if upcast_clause:
        entry["upcast_clause"] = upcast_clause
    if delayed:
        entry["delayed"] = delayed
    if agent_clauses:
        entry["agent_clauses"] = agent_clauses
    if notes:
        entry["notes"] = " ".join(notes)

    return entry


def make_area(area: dict) -> dict:
    shape = area["shape"]
    if shape == "cone":
        return {"cone": {"size": area["size"]}}
    if shape == "line":
        return {"line": {"length": area["length"], "width": area["width"]}}
    if shape == "sphere":
        return {"sphere": {"radius": area["radius"]}}
    if shape == "cube":
        return {"cube": {"size": area["size"]}}
    if shape == "emanation":
        return {"emanation": {"radius": area["radius"]}}
    if shape == "cylinder":
        return {"cylinder": {"radius": area["radius"], "height": area["height"]}}
    raise ValueError(f"unknown area shape {shape}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Scaffold spell effect entries from SRD data.")
    parser.add_argument("ids", nargs="*", help="SRD spell index values to scaffold.")
    parser.add_argument("--party", action="store_true", help="Scaffold every spell in data/party/standard-spells.json.")
    parser.add_argument("--monster-core", type=int, nargs="?", const=45, help="Scaffold the N most-referenced SRD monster spells.")
    args = parser.parse_args(argv)

    spells = load_spells()

    ids: set[str] = set(args.ids or [])
    if args.party:
        ids.update(load_party_spell_ids())
    if args.monster_core is not None:
        ids.update(load_monster_core_spell_ids(args.monster_core))

    ids = sorted(ids)

    if not ids:
        print("error: no spell ids provided", file=sys.stderr)
        return 1

    missing = [idx for idx in ids if idx not in spells]
    if missing:
        print(f"error: unknown spell ids: {missing}", file=sys.stderr)
        return 1

    result = {idx: scaffold(spells[idx]) for idx in ids}
    print(json.dumps(result, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
