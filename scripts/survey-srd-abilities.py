#!/usr/bin/env python3
"""Survey recurring ability patterns in data/srd/2024/5e-SRD-Monsters.json.

Usage: python3 -I scripts/survey-srd-abilities.py [--json | --md | --extras | --unmatched | --unmodeled | --prose | --sample ID]

Every ability entry (trait, action, bonus action, reaction, legendary action)
gets zero or more pattern tags from the rules below. Rules read the structured
fields (usage, dc, damage, spellcasting, multiattack_type, action_options) and
the desc text. Tags overlap on purpose: one ability can be a recharge breath
weapon and a save-or-condition at once. Counts are per ability entry, with the
number of distinct monsters and distinct ability names beside them.
"""
import collections
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MONSTERS = ROOT / "data/srd/2024/5e-SRD-Monsters.json"
SECTIONS = ["special_abilities", "actions", "bonus_actions", "reactions", "legendary_actions"]
SECTION_LABEL = {
    "special_abilities": "trait",
    "actions": "action",
    "bonus_actions": "bonus action",
    "reactions": "reaction",
    "legendary_actions": "legendary action",
}
CONDITIONS = [
    "Blinded", "Charmed", "Deafened", "Exhaustion", "Frightened", "Grappled",
    "Incapacitated", "Invisible", "Paralyzed", "Petrified", "Poisoned", "Prone",
    "Restrained", "Stunned", "Unconscious",
]
COND_RE = "|".join(CONDITIONS)
SHAPES = ["Cone", "Line", "Emanation", "Sphere", "Cube", "Cylinder"]


def has(rx, text, flags=re.I):
    return re.search(rx, text, flags) is not None


def inflicted_conditions(desc):
    """Conditions the ability puts on a target, not ones it merely checks for."""
    found = set()
    for m in re.finditer(
        rf"(?:has|have|gains?|becomes|is|are|with) (?:the )?((?:{COND_RE})(?:(?:,| and| or) (?:{COND_RE}))*)(?: conditions?)?",
        desc,
    ):
        chunk = m.group(0)
        before = desc[: m.start()].rstrip()
        tail = desc[m.end(): m.end() + 12]
        if re.match(r"\s+by\b", tail):
            continue
        # prerequisites and negations: "that is Charmed", "doesn't have the Incapacitated condition"
        if re.search(r"\b(?:unless|while|if)\b[^.,;]{0,30}$", before):
            continue
        if re.search(r"(?:n.t|\bnot|\bunless|\bwithout|\bthat|\bwho|\bwhich|\bwhile it|\bwhile the \w+|\bif (?:the \w+|it|a creature|the creature))$", before):
            continue
        for c in CONDITIONS:
            if c in chunk:
                found.add(c)
    return found


def attack_roll(a):
    return has(r"^(Melee|Ranged)( or (Melee|Ranged))? Attack Roll", a["desc"], 0)


def save_in(a):
    return "Saving Throw: DC" in a["desc"]


def hit_text(desc):
    i = desc.find("Hit:")
    return desc[i:] if i >= 0 else ""


def after_save(desc):
    i = desc.find("Failure")
    return desc[i:] if i >= 0 else desc


# Each rule: (id, group, label, mechanical?, predicate(monster, section, ability))
RULES = []


def rule(pid, group, label, mech, fn):
    RULES.append((pid, group, label, mech, fn))


# ---- multiattack
def ma_types(a):
    return [x["type"] for x in a.get("actions", [])]


rule("MA1", "Multiattack", "Multiattack: one attack repeated N times", True,
     lambda m, s, a: a["name"] == "Multiattack" and len(a.get("actions", [])) == 1 and "action_options" not in a)
rule("MA2", "Multiattack", "Multiattack: two or more different attacks in a fixed sequence", True,
     lambda m, s, a: a["name"] == "Multiattack" and len(a.get("actions", [])) >= 2 and "action_options" not in a)
rule("MA3", "Multiattack", "Multiattack with a choice between options (action_options)", True,
     lambda m, s, a: a["name"] == "Multiattack" and "action_options" in a)
rule("MA4", "Multiattack", "Multiattack that includes a non-attack action (special type) or a spell", True,
     lambda m, s, a: a["name"] == "Multiattack" and (
         "special" in ma_types(a)
         or any(o.get("type") == "special" for o in ((a.get("action_options") or {}).get("from") or {}).get("options", []))
         or has(r"\b(uses?|casts?|cast)\b", a["desc"])))
rule("MA5", "Multiattack", "Multiattack that lets the monster swap an attack for a spell or other action", True,
     lambda m, s, a: a["name"] == "Multiattack" and has(r"replace|in place of|instead of", a["desc"]))
rule("MA6", "Multiattack", "Multiattack whose structured fields are empty (prose only)", False,
     lambda m, s, a: a["name"] == "Multiattack" and not a.get("actions") and "action_options" not in a)

# ---- attacks
rule("AT1", "Attack roll", "Attack roll with one damage entry (or flat 1 damage) and no rider", True,
     lambda m, s, a: attack_roll(a) and len(a.get("damage", [])) <= 1
     and not save_in(a) and not inflicted_conditions(hit_text(a["desc"]))
     and not has(r"\b(pushed|pulled)\b|Swallow|cursed", hit_text(a["desc"])))
rule("AT2", "Attack roll", "Attack roll with a second damage entry (extra damage of another type or dice)", True,
     lambda m, s, a: attack_roll(a) and len(a.get("damage", [])) >= 2)
rule("AT3", "Attack roll", "Attack roll that applies a condition on hit", True,
     lambda m, s, a: attack_roll(a) and bool(inflicted_conditions(hit_text(a["desc"]))))
rule("AT4", "Attack roll", "Attack roll followed by a saving throw rider", True,
     lambda m, s, a: attack_roll(a) and save_in(a))
rule("AT5", "Attack roll", "Attack roll with forced movement on hit", True,
     lambda m, s, a: attack_roll(a) and has(r"\b(pushed|pulled)\b", hit_text(a["desc"])))
rule("AT6", "Attack roll", "Melee or Ranged attack roll (thrown weapon or both modes)", True,
     lambda m, s, a: has(r"^Melee or Ranged Attack Roll", a["desc"], 0))
rule("AT7", "Attack roll", "Attack roll with a curse or lasting affliction on hit (lycanthropy, mummy rot)", True,
     lambda m, s, a: attack_roll(a) and has(r"\bcursed\b", hit_text(a["desc"])))
rule("AT8", "Attack roll", "Attack roll that kills or converts a target reduced to 0 Hit Points", True,
     lambda m, s, a: attack_roll(a) and has(r"reduced to 0 Hit Points", hit_text(a["desc"])))

# ---- saves
rule("SV1", "Saving throw", "Save for damage, half on success", True,
     lambda m, s, a: save_in(a) and not attack_roll(a) and bool(a.get("damage")) and has(r"Success:\s*Half damage", a["desc"]))
rule("SV2", "Saving throw", "Save for damage, nothing on success", True,
     lambda m, s, a: save_in(a) and not attack_roll(a) and bool(a.get("damage")) and not has(r"Success:\s*Half damage", a["desc"]))
rule("SV3", "Saving throw", "Save or condition (non-attack ability)", True,
     lambda m, s, a: save_in(a) and not attack_roll(a) and bool(inflicted_conditions(after_save(a["desc"]))))
rule("SV4", "Saving throw", "Save with forced movement on failure", True,
     lambda m, s, a: save_in(a) and has(r"\b(pushed|pulled|moves? the target|swaps? places)\b", after_save(a["desc"])))
rule("SV5", "Saving throw", "Save with a repeat save or end-of-turn save to end the effect", True,
     lambda m, s, a: save_in(a) and has(r"repeats the save|repeat the save|at the end of each of its turns|at the end of each of the target", a["desc"]))
rule("SV6", "Saving throw", "Save where success grants immunity for a time", True,
     lambda m, s, a: save_in(a) and has(r"immune to .{0,60}for 24 hours|immune to this .{0,30}(Stench|Moan|Gaze|Roar|Presence|Visage)", a["desc"]))
rule("SV7", "Saving throw", "Save with a Failure or Success branch beyond damage and conditions (stat penalty, curse, transport, custom)", True,
     lambda m, s, a: save_in(a) and not a.get("damage") and not inflicted_conditions(after_save(a["desc"])))

rule("SV8", "Saving throw", "Two-stage save ('First Failure' then 'Second Failure')", True,
     lambda m, s, a: has(r"First Failure:", a["desc"], 0))
rule("SV9", "Saving throw", "Save with a damage or condition dependent on the target's current Hit Points or type (20 HP or fewer, Humanoid only, Undead excluded)", True,
     lambda m, s, a: save_in(a) and has(r"Hit Points or fewer|isn.t an? (Undead|Construct)|that isn.t|only if|Celestials, Fiends", a["desc"], 0))

# ---- area shapes
rule("AR1", "Area shape", "Cone", True, lambda m, s, a: has(r"\d+-foot Cone\b", a["desc"], 0))
rule("AR2", "Area shape", "Line", True, lambda m, s, a: has(r"-foot-?\s?(long, \d+-foot-)?wide Line\b|-footwide Line\b", a["desc"], 0))
rule("AR3", "Area shape", "Emanation", True, lambda m, s, a: has(r"\d+-foot Emanation\b", a["desc"], 0))
rule("AR4", "Area shape", "Sphere", True, lambda m, s, a: has(r"-foot-radius Sphere\b", a["desc"], 0))
rule("AR5", "Area shape", "Cube", True, lambda m, s, a: has(r"-foot Cube\b", a["desc"], 0))
rule("AR6", "Area shape", "Cylinder", True, lambda m, s, a: has(r"Cylinder\b", a["desc"], 0))
rule("AR7", "Area shape", "Any area template (union of the six shapes)", True,
     lambda m, s, a: has(r"\d+-foot Cone\b|wide Line\b|-footwide Line\b|\d+-foot Emanation\b|-foot-radius Sphere\b|-foot Cube\b|Cylinder\b", a["desc"], 0))

# ---- usage limits and breath
rule("BR1", "Usage limit", "Breath weapon (name ends in Breath, or desc says exhales)", True,
     lambda m, s, a: s != "special_abilities" and (has(r"Breath$", a["name"]) or has(r"\bexhales?\b", a["desc"])))
rule("US1", "Usage limit", "Recharge on a die roll (Recharge 5-6, 6, 4-6)", True,
     lambda m, s, a: a.get("usage", {}).get("type") == "recharge on roll")
rule("US2", "Usage limit", "Limited uses per day", True,
     lambda m, s, a: a.get("usage", {}).get("type") == "per day")
rule("US3", "Usage limit", "Recharge after a short or long rest", True,
     lambda m, s, a: a.get("usage", {}).get("type") == "recharge after rest")
rule("US4", "Usage limit", "Legendary action cooldown ('can't take this action again until the start of its next turn')", True,
     lambda m, s, a: has(r"can.t take this action again", a["desc"]))
rule("US5", "Usage limit", "Prose-only usage limit with no usage field (once per turn, until a rest)", True,
     lambda m, s, a: "usage" not in a and not has(r"can.t take this action again", a["desc"])
     and has(r"once per turn|once on each of its turns|can.t (use|do) this .{0,20}again|must finish a (Short or )?Long Rest", a["desc"]))

# ---- spellcasting
rule("SP1", "Spellcasting", "Spellcasting block with a structured spell list (spells, slots, or at-will lists)", True,
     lambda m, s, a: "spellcasting" in a and bool(a["spellcasting"].get("spells")))
rule("SP2", "Spellcasting", "Names a spell to cast through a trait, action or legendary action without its own list", True,
     lambda m, s, a: a["name"] != "Multiattack" and ("spellcasting" not in a or not a["spellcasting"].get("spells"))
     and has(r"\bcasts? (the )?(one of the following spells|[A-Z][a-z]+)|uses Spellcasting to cast|can cast the|requiring no (spell )?components", a["desc"], 0))

# ---- named conditions inflicted
for c in ["Grappled", "Restrained", "Prone", "Incapacitated", "Poisoned", "Frightened", "Blinded", "Paralyzed", "Unconscious", "Charmed", "Deafened", "Petrified", "Stunned", "Invisible"]:
    rule(f"CN-{c[:4]}", "Condition inflicted", f"Inflicts {c}", True,
         (lambda cc: lambda m, s, a: cc in inflicted_conditions(a["desc"]))(c))
rule("CN-Exha", "Condition inflicted", "Inflicts Exhaustion levels", True,
     lambda m, s, a: has(r"Exhaustion level", a["desc"], 0))

# ---- grapple, swallow, summon, forced movement, auras
rule("GR1", "Grapple and swallow", "Grapple with an escape DC (on hit or on failed save)", True,
     lambda m, s, a: has(r"escape DC", a["desc"]))
rule("GR2", "Grapple and swallow", "Swallow or engulf", True,
     lambda m, s, a: a["name"] in ("Swallow", "Engulf"))
rule("GR3", "Grapple and swallow", "Ability that works on an already Grappled or Restrained target", True,
     lambda m, s, a: has(r"(Grappled|Restrained) by (the|its)|it is grappling|creature it has Grappled|Grappled creature|Restrained by its", a["desc"]))
rule("SM1", "Summon, spawn and animate", "Creates or animates other creatures (Split, Create Specter, Animate Trees, spawn)", True,
     lambda m, s, a: has(r"splits into|animates up to|as a Specter|turns into a Troll|rises the following sunset|becomes a Were", a["desc"], 0))
rule("OB1", "Summon, spawn and animate", "Creates an object or hazard with its own AC and HP (webs, tentacles, ropes, bags)", True,
     lambda m, s, a: has(r"\(AC \d+; HP \d+|has AC \d+, HP \d+|AC \d+, HP \d+", a["desc"], 0))
rule("FM1", "Forced movement", "Pushes, pulls or drags a creature (any source)", True,
     lambda m, s, a: has(r"\b(pushed|pulled)\b|\bpulls one creature|moves? the target|swaps? places|Reel", a["desc"] + " " + a["name"]))
rule("AU1", "Aura and zone", "Passive emanation aura on a trait (acts on creatures near the monster each turn)", True,
     lambda m, s, a: s == "special_abilities" and has(r"Emanation", a["desc"], 0))
rule("AU2", "Aura and zone", "Zone placed in the world by an action (whirlwind, cloud, ink)", True,
     lambda m, s, a: s != "special_abilities" and has(r"lasts until|fills a|cloud .{0,30}(lasts|remains)|Heavily Obscured", a["desc"]))
rule("AU3", "Aura and zone", "Retaliation or equipment damage when a creature touches or hits the monster", True,
     lambda m, s, a: s == "special_abilities" and (a["name"] in ("Corrosive Form", "Barbed Hide", "Fire Form") or (has(r"(hits|touches|grapples|ends its turn|first touches|starts its turn) .{0,80}takes? \d+ \(", a["desc"]) and not has(r"Emanation", a["desc"], 0))))

# ---- defenses
rule("DF1", "Defense", "Magic Resistance (advantage on saves against spells)", True,
     lambda m, s, a: a["name"] in ("Magic Resistance", "Greater Magic Resistance"))
rule("DF2", "Defense", "Legendary Resistance (turn a failed save into a success, per day)", True,
     lambda m, s, a: a["name"] == "Legendary Resistance")
rule("DF4", "Defense", "Healing: regeneration, absorb damage to heal, drain to heal", True,
     lambda m, s, a: has(r"Regeneration|Absorption", a["name"]) or has(r"(?<!can.t )(?<!cannot )\bregains? (a number of |\d+ |all |Hit Points equal)", a["desc"]))
rule("DF5", "Defense", "Revival: returns after being destroyed (Restoration, Spirit Jar)", True,
     lambda m, s, a: has(r"gains a new body|returns to life|reviving with all|revives with all|reforms in", a["desc"]))
rule("DF6", "Defense", "Death trigger: burst, instant effect, or stays at 1 HP (Death Burst, Death Throes, Undead Fortitude)", True,
     lambda m, s, a: a["name"] in ("Death Burst", "Death Throes", "Undead Fortitude") or has(r"explodes when it dies", a["desc"]))
rule("DF7", "Defense", "Reaction that boosts AC, reduces damage, redirects or negates an attack", True,
     lambda m, s, a: s == "reactions" and has(r"\bAC\b|Armor Class|redirect|reduces the damage|reduce the damage|causes the attack to miss|Disadvantage", a["desc"]))
rule("DF8", "Defense", "Form-based immunity or defense by trait name (Immutable Form, Reflective Carapace, Reactive Heads, Blurred Form)", True,
     lambda m, s, a: a["name"] in ("Immutable Form", "Reflective Carapace", "Reactive Heads", "Blurred Form", "Evasion", "Reactive", "Multiple Heads"))
rule("DF9", "Defense", "Temporary Hit Points or AC bonus granted to a creature", True,
     lambda m, s, a: has(r"Temporary Hit Points|AC increases", a["desc"]))

rule("AC1", "Action chaining", "Legendary action, reaction or bonus action that just runs another of the monster's own actions", True,
     lambda m, s, a: s in ("legendary_actions", "reactions", "bonus_actions") and has(r"^(The \w[\w ]*?|It) (makes one|makes a|uses|makes an?) [A-Z][\w ]*( attack| or [A-Z][\w ]* attack)?\.?( The \w+ can.t take this action again[\w ]*\.)?$", a["desc"], 0)
     or (s == "reactions" and has(r"Response: The \w+ uses [A-Z]\w+\.$", a["desc"], 0)))

# ---- roll modifiers
rule("AD1", "Roll modifier", "Conditional advantage or disadvantage in a trait (Pack Tactics, Sunlight Sensitivity, Blood Frenzy)", True,
     lambda m, s, a: s == "special_abilities" and has(r"\bAdvantage\b|\bDisadvantage\b", a["desc"]) and a["name"] not in ("Magic Resistance", "Greater Magic Resistance"))
rule("AD2", "Roll modifier", "Damage multiplier or conditional damage bonus (Siege Monster, Bloodied Fury, Berserk)", True,
     lambda m, s, a: s == "special_abilities" and has(r"double damage|extra .{0,20}damage|Bloodied", a["desc"]))

# ---- movement
rule("MV1", "Movement", "Self teleport (Teleport, Misty Step, Tree Stride, Radiant Teleport, Ethereal Jaunt)", True,
     lambda m, s, a: has(r"\bteleports?\b|Misty Step|Ethereal Jaunt|Tree Stride|enters the Ethereal", a["desc"]) or has(r"Teleport|Misty|Jaunt|Stride|Vanish", a["name"]))
rule("MV2", "Movement", "Move then attack or move with a rider (Pounce, Charge, Trample, Rampage, Swoop)", True,
     lambda m, s, a: a["name"] in ("Pounce", "Charge", "Trample", "Trampling Charge", "Rampage", "Swoop", "Aquatic Charge", "Onslaught", "Charging Horn", "Cloaked Flight", "World-Shaking Movement", "Bubble Dash", "Prowl") or has(r"moves up to .{0,60}(makes|and makes)", a["desc"]))
rule("MV3", "Movement", "Bonus-action Disengage, Hide, Dash or leap (Nimble Escape, Cunning Action, Leap)", True,
     lambda m, s, a: s == "bonus_actions" and has(r"Disengage|\bHide\b|\bDash\b|\bLeap\b|jump", a["desc"] + " " + a["name"]))
rule("MV4", "Movement", "Passive terrain or space traversal (Spider Climb, Web Walker, Ice Walk, Earth Glide, Tunneler, Standing Leap)", True,
     lambda m, s, a: s == "special_abilities" and a["name"] in ("Spider Climb", "Web Walker", "Ice Walk", "Earth Glide", "Tunneler", "Incorporeal Movement", "Standing Leap", "Running Leap", "Jumper", "Air Form", "Fire Form", "Water Form", "Amorphous", "Compression", "Swarm", "Ooze Cube", "Aberrant Ground"))
rule("MV5", "Movement", "No Opportunity Attack when moving (Flyby, Agile)", True,
     lambda m, s, a: has(r"doesn.t provoke an Opportunity Attack", a["desc"]))
rule("MV6", "Movement", "Speed change imposed on a target (Slowed, Speed halved or 0)", True,
     lambda m, s, a: has(r"Speed (is )?(halved|reduced)|Speed is 0|Speed to 0|reduces .{0,20}Speed", a["desc"]))

# ---- transformation
rule("TF1", "Transformation", "Shape-Shift and alternate forms (Shape-Shift, Succubus Form, Misty Escape)", True,
     lambda m, s, a: a["name"] in ("Shape-Shift", "Succubus Form", "Incubus Form", "Misty Escape", "Draconic Origin") or has(r"shape-?shifts? into|uses Shape-Shift", a["desc"]))
rule("TF2", "Transformation", "Invisibility, ethereal travel, or hiding (Invisibility, Vanish, Shadow Stealth)", True,
     lambda m, s, a: has(r"^(Invisibility|Vanish|Shadow Stealth|Ethereal Jaunt|Etherealness|Cloaked Flight|Prowl|Nightmare)$", a["name"]) or has(r"\bbecomes? Invisible|has the Invisible condition|Invisibility", a["desc"]))
rule("TF3", "Transformation", "Possession or mind-control of a target", True,
     lambda m, s, a: has(r"possess|under its control|acts as an ally", a["desc"]))

# ---- utility
rule("UT1", "Utility", "Breathing and environment (Amphibious, Water Breathing, Hold Breath)", False,
     lambda m, s, a: a["name"] in ("Amphibious", "Water Breathing", "Hold Breath", "Limited Amphibiousness"))
rule("UT2", "Utility", "Light emission or light sensitivity (Illumination, Sunlight Sensitivity)", True,
     lambda m, s, a: has(r"Illumination|Sunlight Sens|Sunlight Weak|Light Sensitivity|Horror Nimbus|^Sunlight$", a["name"]))
rule("UT3", "Utility", "Telepathy, extra senses, speech and communication traits", False,
     lambda m, s, a: a["name"] in ("Probing Telepathy", "Telepathic Bond", "Shark Telepathy", "Sense Magic", "Iron Scent", "Treasure Sense", "Speak with Beasts and Plants", "Mimicry", "Ethereal Sight", "Divine Awareness", "Inscrutable", "Gibbering", "Shielded Mind", "Vampiric Connection"))
rule("UT4", "Utility", "Entry, placement and vulnerability restrictions (Forbiddance, Running Water, Stake to the Heart, Water Susceptibility)", True,
     lambda m, s, a: a["name"] in ("Forbiddance", "Running Water", "Stake to the Heart", "Vampire Weakness", "Water Susceptibility", "Aversion to Fire"))
rule("UT5", "Utility", "Bespoke one-off rules with no recurring mechanic (Magic Rope, Wishes, Spirit Jar, Soul Bag, Spell Storing)", False,
     lambda m, s, a: a["name"] in ("Siege Monster", "Magic Rope", "Wishes", "Spirit Jar", "Soul Bag", "Spell Storing", "Troll Spawn", "Ooze Cube", "Vampiric Connection", "Bound", "Confer Fire Resistance", "Adhesive (Object Form Only)", "Freeze", "Loathsome Limbs", "Coven Magic", "Abduct"))


rule("RX1", "Structure", "Reaction or ability written as 'Trigger: ... Response: ...'", True,
     lambda m, s, a: has(r"^Trigger:", a["desc"], 0))

ENGINE = {
    # E = fits an effects vocabulary (rolls, saves, conditions, damage, recharge, movement, areas)
    # S = delegate to the spell system (data/srd/2024/5e-SRD-Spells.json)
    # F = boolean flag or small modifier the engine reads, no roll of its own
    # U = leave unmodeled (state outside combat, entity creation, persistent zones, pure prose)
    "MA1": "E", "MA2": "E", "MA3": "E", "MA4": "E", "MA5": "E", "MA6": "U",
    "AT1": "E", "AT2": "E", "AT3": "E", "AT4": "E", "AT5": "E", "AT6": "E", "AT7": "U", "AT8": "U",
    "SV1": "E", "SV2": "E", "SV3": "E", "SV4": "E", "SV5": "E", "SV6": "E", "SV7": "U", "SV8": "E", "SV9": "E",
    "AR1": "E", "AR2": "E", "AR3": "E", "AR4": "E", "AR5": "E", "AR6": "E", "AR7": "E",
    "BR1": "E", "US1": "E", "US2": "E", "US3": "E", "US4": "E", "US5": "U",
    "SP1": "S", "SP2": "S",
    "GR1": "E", "GR2": "E", "GR3": "E", "SM1": "U", "FM1": "E",
    "AU1": "E", "AU2": "U", "AU3": "E", "OB1": "U",
    "DF1": "F", "DF2": "E", "DF4": "E", "DF5": "U", "DF6": "E", "DF7": "E", "DF8": "F", "DF9": "E",
    "AD1": "E", "AD2": "E",
    "MV1": "E", "MV2": "E", "MV3": "E", "MV4": "F", "MV5": "F", "MV6": "E",
    "TF1": "U", "TF2": "E", "TF3": "U",
    "UT1": "F", "UT2": "F", "UT3": "U", "UT4": "F", "UT5": "U",
    "AC1": "E", "RX1": "E",
}
for _p, *_ in RULES:
    if _p.startswith("CN-"):
        ENGINE[_p] = "E"
ENGINE_ORDER = {"E": 0, "S": 1, "F": 2, "U": 3}


def entries(monsters):
    for m in monsters:
        for s in SECTIONS:
            for a in m.get(s) or []:
                yield m, s, a


def classify(monsters):
    tagged = []
    for m, s, a in entries(monsters):
        tags = [r[0] for r in RULES if r[4](m, s, a)]
        tagged.append((m, s, a, tags))
    return tagged


def mechanical_signal(a):
    """True when the entry has structured data or text the engine could compute with.

    Counts as mechanical: a structured field (dc, damage, attack_bonus, usage,
    spellcasting, actions), a dice expression, a save or attack roll, advantage
    or disadvantage, a named condition, a damage or healing clause, a movement
    or teleport clause, an area shape, or a resistance or immunity clause.
    Anything else is prose-only.
    """
    d = a["desc"]
    if any(k in a for k in ("dc", "damage", "attack_bonus", "usage", "spellcasting", "actions", "action_options")):
        return True
    return has(
        r"\d+d\d+|\(\d+\)|Saving Throw|Attack Roll|\bAdvantage\b|\bDisadvantage\b"
        r"|\b(Blinded|Charmed|Deafened|Frightened|Grappled|Incapacitated|Invisible|Paralyzed|Petrified|Poisoned|Prone|Restrained|Stunned|Unconscious)\b"
        r"|\bSpeed\b|\bspeed\b|\bteleport|Emanation|Cone\b|Sphere\b|Cube\b|Cylinder\b|Line\b"
        r"|[Rr]esistance|immune|[Vv]ulnerab|regains? |Hit Points|damage|Temporary|Opportunity Attack|\bAC\b|d20|\bcast",
        d, 0)


def kind_of(st):
    """Mechanical, Flag, Spell, Prose-only, or Mechanical-but-stateful, from the engine class and the share of entries with computable text."""
    sig = st["signal_share"]
    if sig is None:
        return "none found"
    pct = f"{round(sig * 100)}%"
    if st["engine"] == "F":
        return f"Flag ({pct} computable)"
    if st["engine"] == "S":
        return f"Mechanical, via spell data ({pct})"
    if st["engine"] == "U":
        return f"Prose-only ({pct})" if sig < 0.5 else f"Mechanical but stateful ({pct})"
    return f"Mechanical ({pct})"


def extras(monsters, tagged):
    """Cross-tabs quoted in the findings file."""
    tag = {id(a): set(tg) for _, _, a, tg in tagged}
    ents = [(m, s, a) for m, s, a, _ in tagged]
    out = {}
    out["save_abilities"] = sum(1 for _, _, a in ents if save_in(a))
    out["attack_roll_abilities"] = sum(1 for _, _, a in ents if attack_roll(a))
    out["save_and_attack_roll"] = sum(1 for _, _, a in ents if save_in(a) and attack_roll(a))
    out["breath_and_recharge"] = sum(1 for _, _, a in ents if {"BR1", "US1"} <= tag[id(a)])
    out["breath_without_usage_field"] = sorted({f'{m["name"]}: {a["name"]}' for m, _, a in ents if "BR1" in tag[id(a)] and "usage" not in a})
    out["breath_not_recharge"] = sum(1 for _, _, a in ents if "BR1" in tag[id(a)] and "US1" not in tag[id(a)])
    out["recharge_not_breath"] = sum(1 for _, _, a in ents if "US1" in tag[id(a)] and "BR1" not in tag[id(a)])
    out["recharge_with_area"] = sum(1 for _, _, a in ents if {"US1", "AR7"} <= tag[id(a)])
    out["recharge_with_save"] = sum(1 for _, _, a in ents if "US1" in tag[id(a)] and save_in(a))
    out["recharge_dice"] = dict(collections.Counter(
        f'{a["usage"].get("dice")} min {a["usage"].get("min_value")}' for _, _, a in ents if a.get("usage", {}).get("type") == "recharge on roll"))
    out["per_day_times"] = dict(collections.Counter(
        a["usage"].get("times") for _, _, a in ents if a.get("usage", {}).get("type") == "per day"))
    out["per_day_excluding_legendary_resistance"] = sum(
        1 for _, _, a in ents if a.get("usage", {}).get("type") == "per day" and a["name"] != "Legendary Resistance")
    names = collections.Counter(a["name"] for _, _, a in ents)
    owners = collections.defaultdict(set)
    for m, _, a in ents:
        owners[a["name"]].add(m["index"])
    out["names_on_one_monster"] = sum(1 for n in names if len(owners[n]) == 1)
    out["names_on_two_or_more"] = sum(1 for n in names if len(owners[n]) >= 2)
    out["top_names"] = names.most_common(12)
    out["monsters_with_legendary_actions"] = sum(1 for m in monsters if m.get("legendary_actions"))
    out["monsters_with_reactions"] = sum(1 for m in monsters if m.get("reactions"))
    out["monsters_with_bonus_actions"] = sum(1 for m in monsters if m.get("bonus_actions"))
    out["monsters_with_spellcasting_block"] = sum(1 for m in monsters if any("spellcasting" in a for s in SECTIONS for a in m.get(s) or []))
    out["monsters_with_multiattack"] = sum(1 for m in monsters if any(a["name"] == "Multiattack" for a in m.get("actions") or []))
    out["monsters_with_no_special_abilities_key"] = sum(1 for m in monsters if not m.get("special_abilities"))
    out["monsters_with_no_abilities_at_all"] = sum(1 for m in monsters if not any(m.get(s) for s in SECTIONS))
    out["attack_entries_with_empty_damage"] = sum(1 for _, _, a in ents if attack_roll(a) and not a.get("damage"))
    return out


def main():
    args = sys.argv[1:]
    monsters = json.loads(MONSTERS.read_text())
    tagged = classify(monsters)
    rules = {r[0]: r for r in RULES}
    stats = {}
    for pid in rules:
        rows = [(m, s, a) for m, s, a, t in tagged if pid in t]
        stats[pid] = {
            "engine": ENGINE[pid],
            "signal_share": round(sum(1 for _, _, a in rows if mechanical_signal(a)) / len(rows), 2) if rows else None,
            "entries": len(rows),
            "monsters": len({m["index"] for m, _, _ in rows}),
            "names": len({a["name"] for _, _, a in rows}),
            "sections": dict(collections.Counter(SECTION_LABEL[s] for _, s, _ in rows)),
            "examples": [],
        }
        by_name = collections.Counter(a["name"] for _, _, a in rows)
        picked, names_seen, mons_seen = [], set(), set()
        for m, s, a in rows:
            if a["name"] not in names_seen and len(picked) < 3:
                names_seen.add(a["name"])
                mons_seen.add(m["name"])
                picked.append(f'{m["name"]}: {a["name"]}')
        for m, s, a in rows:
            if len(picked) >= 3:
                break
            if m["name"] not in mons_seen:
                mons_seen.add(m["name"])
                picked.append(f'{m["name"]}: {a["name"]}')
        stats[pid]["examples"] = picked
        stats[pid]["top_names"] = by_name.most_common(5)
    unmatched = [(m, s, a) for m, s, a, t in tagged if not t]

    def best(tags):
        return min((ENGINE[x] for x in tags if x != "RX1"), default="U", key=lambda c: ENGINE_ORDER[c]) if tags else "U"

    entry_class = collections.Counter(best(t) for _, _, _, t in tagged)
    only_u = [(m, s, a) for m, s, a, t in tagged if best(t) == "U"]
    by_section_class = collections.defaultdict(collections.Counter)
    for _, s, _, t in tagged:
        by_section_class[SECTION_LABEL[s]][best(t)] += 1
    summary = {
        "monsters": len(monsters),
        "entries": len(tagged),
        "entries_by_section": dict(collections.Counter(SECTION_LABEL[s] for _, s, _, _ in tagged)),
        "distinct_names": len({a["name"] for _, _, a, _ in tagged}),
        "untagged": len(unmatched),
        "tagged_with_any": len(tagged) - len(unmatched),
        "entries_by_best_engine_class": dict(entry_class),
        "engine_class_by_section": {k: dict(v) for k, v in by_section_class.items()},
        "statblock_fields": {
            k: sum(1 for m in monsters if m.get(k))
            for k in ("damage_resistances", "damage_vulnerabilities", "damage_immunities", "condition_immunities", "forms", "gear")
        },
        "usage_types": dict(collections.Counter(a["usage"]["type"] for _, _, a, _ in tagged if "usage" in a)),
        "no_mechanical_signal": sum(1 for _, _, a, _ in tagged if not mechanical_signal(a)),
    }
    if "--json" in args:
        print(json.dumps({"summary": summary, "patterns": stats}, indent=1))
        return
    if "--unmatched" in args:
        for m, s, a in unmatched:
            print(f'{m["name"]} | {SECTION_LABEL[s]} | {a["name"]} | {a["desc"][:140]}')
        return
    if "--extras" in args:
        print(json.dumps(extras(monsters, tagged), indent=1))
        return
    if "--md" in args:
        group = None
        for pid, grp, label, mech, _ in RULES:
            st = stats[pid]
            if grp != group:
                group = grp
                print(f"\n### {grp}\n")
                print("| ID | Pattern | Entries | Monsters | Sections | Kind | Examples |")
                print("|---|---|---:|---:|---|---|---|")
            secs = ", ".join(f"{k} {v}" for k, v in sorted(st["sections"].items(), key=lambda kv: -kv[1]))
            ex = "; ".join(st["examples"][:3])
            print(f"| {pid} | {label} | {st['entries']} | {st['monsters']} | {secs} | {kind_of(st)} | {ex} |")
        return
    if "--unmodeled" in args:
        by = collections.defaultdict(list)
        for m, s, a in only_u:
            by[(SECTION_LABEL[s], a["name"])].append(m["name"])
        for (sec, n), ms in sorted(by.items(), key=lambda kv: (-len(kv[1]), kv[0])):
            print(f"{len(ms):3d} {sec:16s} {n} ({', '.join(ms[:3])})")
        return
    if "--prose" in args:
        by = collections.defaultdict(list)
        for m, s, a, t in tagged:
            if not mechanical_signal(a):
                by[a["name"]].append(m["name"])
        for n, ms in sorted(by.items(), key=lambda kv: -len(kv[1])):
            print(f"{len(ms):3d} {n} ({', '.join(ms[:3])})")
        return
    if "--sample" in args:
        pid = args[args.index("--sample") + 1]
        for m, s, a, t in tagged:
            if pid in t:
                print(f'{m["name"]} | {SECTION_LABEL[s]} | {a["name"]} | {a["desc"][:200]}')
        return
    print(json.dumps(summary, indent=1))
    group = None
    for pid, grp, label, mech, _ in RULES:
        if grp != group:
            group = grp
            print(f"\n## {grp}")
        st = stats[pid]
        print(f'{pid:12s} {st["entries"]:4d} entries {st["monsters"]:4d} monsters {st["names"]:3d} names | {st["engine"]} sig={st["signal_share"]} | {label} | {"; ".join(st["examples"])}')


if __name__ == "__main__":
    main()
