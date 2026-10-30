#!/usr/bin/env python3
"""Generate the standard party stat blocks from the committed progression tables.

Usage:
  python3 -I scripts/generate-party.py          # write data/party/standard/*.json
  python3 -I scripts/generate-party.py --check  # exit 1 if committed files differ

Inputs (all committed):
  data/party/progression/{defender,striker,healer,caster}.json
  data/party/standard-spells.json
  data/spells/effects.json
  data/srd/2024/5e-SRD-Spells.json  (casting time only)

Output: 80 stat blocks, `standard-<archetype>-<level:02d>.json`, levels 1 to 20.
The script is deterministic: same inputs, byte-identical outputs.
"""

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PROGRESSION_DIR = ROOT / "data/party/progression"
OUT_DIR = ROOT / "data/party/standard"
STANDARD_SPELLS = ROOT / "data/party/standard-spells.json"
EFFECTS = ROOT / "data/spells/effects.json"
SRD_SPELLS = ROOT / "data/srd/2024/5e-SRD-Spells.json"

ARCHETYPES = ["defender", "striker", "healer", "caster"]
LEVELS = range(1, 21)
SOURCE = "homebrew-bench standard party"
SCHEMA_VERSION = "1"
ABILITY_KEYS = {
    "str": "strength",
    "dex": "dexterity",
    "con": "constitution",
    "int": "intelligence",
    "wis": "wisdom",
    "cha": "charisma",
}
ABILITY_SHORT = {v: k for k, v in ABILITY_KEYS.items()}
SKILL_ABILITY = {
    "perception": "wisdom",
    "medicine": "wisdom",
    "stealth": "dexterity",
    "arcana": "intelligence",
}


def proficiency_bonus(level):
    return 2 + (level - 1) // 4


def modifier(score):
    return (score - 10) // 2


def step_at(steps, level):
    """Last step whose `from` is at or below `level`, or None."""
    current = None
    for step in steps:
        if step["from"] <= level:
            current = step
    return current


def merged_abilities(steps, level):
    scores = {}
    for step in steps:
        if step["from"] <= level:
            scores.update({k: v for k, v in step.items() if k != "from"})
    return scores


def dice_with_modifier(count, die, mod):
    if mod == 0:
        return f"{count}d{die}"
    return f"{count}d{die}{mod:+d}"


def hit_points(table, level, con_mod):
    die = table["hit_die"]
    average = die + (level - 1) * (die // 2 + 1) + level * con_mod
    return {"average": average, "dice": dice_with_modifier(level, die, level * con_mod)}


def attack_ability(attack):
    return attack["ability"]


def build_attack(attack, level, scores, pb):
    mod = modifier(scores[attack_ability(attack)])
    bonus_step = step_at(attack.get("attack_bonus", []), level)
    to_hit = pb + mod + (bonus_step["bonus"] if bonus_step else 0)
    damage = []
    base = step_at(attack["damage"], level)
    damage.append({"dice": base["dice"], "type": base["type"], "modifier": mod, "magical": False})
    extra = step_at(attack.get("bonus_damage", []), level)
    if extra:
        damage.append({"dice": extra["dice"], "type": extra["type"], "modifier": 0, "magical": False})
    body = {"modifier": to_hit, "target_count": 1, "damage": damage}
    if "reach" in attack:
        body["reach"] = attack["reach"]
    if "range" in attack:
        body["range"] = attack["range"]
    desc_kind = "Melee" if "reach" in attack else "Ranged"
    return {
        "id": attack["id"],
        "name": attack["name"],
        "desc": f"{desc_kind} weapon attack, {to_hit:+d} to hit.",
        "kind": "modeled",
        "effects": [{"attack": body}],
    }


def build_multiattack(table, level):
    spec = table.get("multiattack")
    if not spec:
        return None
    count = step_at(spec["count"], level)["count"]
    if count <= 1:
        return None
    return {
        "id": "multiattack",
        "name": "Multiattack",
        "desc": f"Makes {count} {spec['attack']} attacks.",
        "kind": "modeled",
        "effects": [
            {"multiattack": {"steps": [{"action": {"action_id": spec["attack"], "count": count}}]}}
        ],
    }


def build_feature(feature, level):
    effects = json.loads(json.dumps(feature["effects"]))
    for effect in effects:
        heal = effect.get("heal")
        if heal and heal.get("modifier") == "level":
            heal["modifier"] = level
    entry = {
        "id": feature["id"],
        "name": feature["name"],
        "desc": feature["desc"],
        "kind": feature["kind"],
        "effects": effects,
    }
    if "usage" in feature:
        entry["usage"] = feature["usage"]
    return entry


def spell_entry(spell_id, name, tier, effects_entry):
    clauses = effects_entry.get("agent_clauses", [])
    if not effects_entry["effects"] and not clauses:
        raise SystemExit(f"error: effects file entry '{spell_id}' has no effects and no agent clauses")
    kind = "modeled" if effects_entry["effects"] and not clauses else "partly_modeled"
    level_text = "Cantrip" if tier == 0 else f"Level {tier} spell"
    entry = {
        "id": spell_id,
        "name": name,
        "desc": f"{level_text}. Cast {name}.",
        "kind": kind,
        "effects": [{"cast": {"spell_id": spell_id, "slot_level": None if tier == 0 else tier, "upcast": False}}],
    }
    if clauses:
        entry["agent_clauses"] = list(clauses)
    if tier == 0:
        entry["usage"] = "at_will"
    return entry


def action_slot(casting_time):
    """Which stat block list a spell belongs in, or None when it is not castable in combat."""
    if casting_time == "Action":
        return "actions"
    if casting_time.startswith("Bonus Action"):
        return "bonus_actions"
    if casting_time.startswith("Reaction"):
        return "reactions"
    return None


def spells_for_level(role, slots, standard, effects, srd_times):
    """Return {list_name: [entries]} and the spell ids skipped for out-of-combat casting time."""
    role_spells = standard[role]
    max_tier = len(slots)
    placed = {"actions": [], "bonus_actions": [], "reactions": []}
    skipped = []
    candidates = []
    for at_will in role_spells["at_will"]:
        candidates.append((0, at_will["id"], at_will["name"]))
    for tier in range(1, max_tier + 1):
        for spell_id, name, _category in role_spells["by_tier"].get(str(tier), []):
            candidates.append((tier, spell_id, name))
    for tier, spell_id, name in candidates:
        if spell_id not in effects:
            raise SystemExit(f"error: standard party spell '{spell_id}' is missing from {EFFECTS.name}")
        casting_time = srd_times.get(spell_id)
        if casting_time is None:
            raise SystemExit(f"error: standard party spell '{spell_id}' is missing from the SRD spell data")
        slot = action_slot(casting_time)
        if slot is None:
            skipped.append(spell_id)
            continue
        placed[slot].append(spell_entry(spell_id, name, tier, effects[spell_id]))
    return placed, skipped


def build_block(table, level, standard, effects, srd_times):
    pb = proficiency_bonus(level)
    scores = merged_abilities(table["ability_scores"], level)
    mods = {name: modifier(score) for name, score in scores.items()}
    ac = step_at(table["armor_class"], level)
    gear = step_at(table["gear"], level)

    saving_throws = {
        short: mods[ABILITY_KEYS[short]] + pb for short in table["saving_throw_proficiencies"]
    }
    skills = {
        skill: mods[SKILL_ABILITY[skill]] + pb for skill in table["skill_proficiencies"]
    }
    passive = 10 + mods["wisdom"] + (pb if "perception" in table["skill_proficiencies"] else 0)

    actions = []
    multiattack = build_multiattack(table, level)
    if multiattack:
        actions.append(multiattack)
    for attack in table["attacks"]:
        actions.append(build_attack(attack, level, scores, pb))
    bonus_actions = []
    traits = []
    reactions = []
    for feature in table.get("features", []):
        if feature["from"] > level:
            continue
        target = {"bonus_actions": bonus_actions, "traits": traits, "actions": actions, "reactions": reactions}[
            feature["slot"]
        ]
        target.append(build_feature(feature, level))

    block = {
        "id": f"standard-{table['archetype']}-{level:02d}",
        "name": f"Standard {table['label']} (Level {level})",
        "source": SOURCE,
        "schema_version": SCHEMA_VERSION,
        "size": table["size"],
        "type": table["creature_type"],
        "alignment": table["alignment"],
        "armor_class": [{"value": ac["value"], "source": ac["source"]}],
        "hit_points": hit_points(table, level, mods["constitution"]),
        "speeds": {"walk": {"feet": table["speed"]}},
        "ability_scores": scores,
        "saving_throws": saving_throws,
        "skills": skills,
        "initiative": mods["dexterity"],
        "proficiency_bonus": pb,
        "senses": {"passive_perception": passive},
        "languages": table["languages"],
        "class": table["label"],
        "level": level,
        "dying": True,
        "gear": gear["items"],
        "notes": [table["class_note"], "No magic items; gear exists only as numbers in this stat block."],
    }
    if not skills:
        del block["skills"]

    casting = table.get("spellcasting")
    if casting:
        slot_step = step_at(casting["slots"], level)
        slots = slot_step["slots"]
        ability = casting["ability"]
        block["spell_slots"] = slots + [0] * (9 - len(slots))
        block["spellcasting"] = {
            "ability": ABILITY_SHORT[ability],
            "save_dc": 8 + pb + mods[ability],
            "attack_bonus": pb + mods[ability],
        }
        placed, _skipped = spells_for_level(casting["spell_list"], slots, standard, effects, srd_times)
        actions.extend(placed["actions"])
        bonus_actions.extend(placed["bonus_actions"])
        reactions.extend(placed["reactions"])

    for key, entries in (
        ("traits", traits),
        ("actions", actions),
        ("bonus_actions", bonus_actions),
        ("reactions", reactions),
    ):
        if entries:
            block[key] = entries
    return block


def load_inputs():
    tables = {a: json.loads((PROGRESSION_DIR / f"{a}.json").read_text(encoding="utf-8")) for a in ARCHETYPES}
    standard = json.loads(STANDARD_SPELLS.read_text(encoding="utf-8"))
    effects = json.loads(EFFECTS.read_text(encoding="utf-8"))
    srd = json.loads(SRD_SPELLS.read_text(encoding="utf-8"))
    srd_times = {spell["index"]: spell["casting_time"] for spell in srd}
    return tables, standard, effects, srd_times


def render(block):
    return json.dumps(block, indent=2, sort_keys=True) + "\n"


def generate():
    tables, standard, effects, srd_times = load_inputs()
    outputs = {}
    for archetype in ARCHETYPES:
        for level in LEVELS:
            block = build_block(tables[archetype], level, standard, effects, srd_times)
            outputs[f"{block['id']}.json"] = render(block)
    return outputs


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--check", action="store_true", help="exit 1 if committed output differs")
    args = parser.parse_args()
    outputs = generate()
    if args.check:
        problems = []
        for name, text in outputs.items():
            path = OUT_DIR / name
            if not path.exists():
                problems.append(f"missing: {path.relative_to(ROOT)}")
            elif path.read_text(encoding="utf-8") != text:
                problems.append(f"differs: {path.relative_to(ROOT)}")
        if OUT_DIR.exists():
            for path in sorted(OUT_DIR.glob("*.json")):
                if path.name not in outputs:
                    problems.append(f"unexpected: {path.relative_to(ROOT)}")
        if problems:
            print("\n".join(problems), file=sys.stderr)
            return 1
        print(f"ok: {len(outputs)} standard party stat blocks match the generator")
        return 0
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    for path in OUT_DIR.glob("*.json"):
        path.unlink()
    for name, text in outputs.items():
        (OUT_DIR / name).write_text(text, encoding="utf-8", newline="\n")
    print(f"wrote {len(outputs)} stat blocks to {OUT_DIR.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
