#!/usr/bin/env python3
"""Convert SRD 5.2.1 monsters into canonical stat blocks, review them, and graduate passing fixtures.

Usage:
  python3 -I scripts/import-srd-monsters.py

Inputs (all committed):
  data/srd/COMMIT
  data/srd/2024/5e-SRD-Monsters.json
  data/srd/corrections.json
  data/srd/2024/5e-SRD-Spells.json
  data/spells/effects.json

Outputs (regenerated on every run):
  data/fixtures/<id>.json      one per monster with zero open blocking entries
  data/srd/import-review.md     review list
  data/srd/import-review.json   JSON twin of the review list

Blocking entries are classes a (structured field contradicts prose), b (structured field
missing or not representable in the closed vocabulary) and c (contradicts an SRD ruleset
table). The script exits 1 while any blocking entry is open. A correction that matches
nothing upstream aborts the run. Prose is read only to detect mismatches, never to produce data,
with two exceptions upstream has no field for: the exact Flyby/Agile sentence becomes no_provoke,
and "can't take this action again until the start of its next turn" becomes
cooldown_until_next_turn with a review note. See ADR 0008.
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
UPSTREAM = ROOT / "data/srd/2024/5e-SRD-Monsters.json"
COMMIT_FILE = ROOT / "data/srd/COMMIT"
CORRECTIONS = ROOT / "data/srd/corrections.json"
FIXTURES = ROOT / "data/fixtures"
REVIEW_MD = ROOT / "data/srd/import-review.md"
REVIEW_JSON = ROOT / "data/srd/import-review.json"
SPELL_EFFECTS = ROOT / "data/spells/effects.json"
UPSTREAM_SPELLS = ROOT / "data/srd/2024/5e-SRD-Spells.json"

SOURCE = "srd-5.2.1"
SCHEMA_VERSION = "1"
SECTIONS = {
    "traits": "special_abilities",
    "actions": "actions",
    "bonus_actions": "bonus_actions",
    "reactions": "reactions",
    "legendary_actions": "legendary_actions",
}
ABILITY_KEYS = {"str": "strength", "dex": "dexterity", "con": "constitution",
                "int": "intelligence", "wis": "wisdom", "cha": "charisma"}
ABILITY_NAMES = {"Strength": "str", "Dexterity": "dex", "Constitution": "con",
                 "Intelligence": "int", "Wisdom": "wis", "Charisma": "cha"}
SIZES = {"tiny", "small", "medium", "large", "huge", "gargantuan"}
DAMAGE_TYPES = {"acid", "bludgeoning", "cold", "fire", "force", "lightning", "necrotic",
                "piercing", "poison", "psychic", "radiant", "slashing", "thunder"}
CONDITIONS = {"blinded", "charmed", "deafened", "exhaustion", "frightened", "grappled",
              "incapacitated", "invisible", "paralyzed", "petrified", "poisoned", "prone",
              "restrained", "stunned", "unconscious"}
# SRD 5.2.1 p. 256, Experience Points by Challenge Rating. CR 0 is printed as "0 or 10".
XP_BY_CR = {0: {0, 10}, 0.125: {25}, 0.25: {50}, 0.5: {100}, 1: {200}, 2: {450}, 3: {700},
            4: {1100}, 5: {1800}, 6: {2300}, 7: {2900}, 8: {3900}, 9: {5000}, 10: {5900},
            11: {7200}, 12: {8400}, 13: {10000}, 14: {11500}, 15: {13000}, 16: {15000},
            17: {18000}, 18: {20000}, 19: {22000}, 20: {25000}, 21: {33000}, 22: {41000},
            23: {50000}, 24: {62000}, 25: {75000}, 26: {90000}, 27: {105000}, 28: {120000},
            29: {135000}, 30: {155000}}
# SRD 5.2.1 p. 256, Proficiency Bonus by Challenge Rating: (highest CR in band, PB).
PB_BANDS = [(4, 2), (8, 3), (12, 4), (16, 5), (20, 6), (24, 7), (28, 8), (30, 9)]
# The SRD gives no legendary action count and the upstream data omits it; SRD rules default to 3.
LEGENDARY_USES_PER_ROUND = 3

DICE_RE = re.compile(r"^(\d+)d(\d+)(?:([+-])(\d+))?$")
ATTACK_RANGED_RE = re.compile(r"Ranged Attack Roll")
DAMAGE_IF_RE = re.compile(r"\s*[A-Za-z]+ damage\s*(?:—|,)?\s*(?:if|unless)\b", re.I)
FLAT_DAMAGE_RE = re.compile(r"\b(\d+) (?!\()([A-Za-z]+) damage")
OTHERWISE_RE = re.compile(r"\botherwise\b", re.I)
RECURRING_RE = re.compile(r"\b(?:start|end) of\b|\beach of\b|\bwhenever\b|\buntil\b|\bwhile\b", re.I)
GRAPPLED_BY_SELF_RE = re.compile(r"\bGrappled by the\b")
PROSE_DICE_RE = re.compile(r"\((\d+d\d+)(?:\s*([+-])\s*(\d+))?\)")
DAMAGE_PHRASE_RE = re.compile(r"\d+ \(\d+d\d+(?:\s*[+-]\s*\d+)?\) [A-Za-z]+ damage")
DAMAGE_CHAIN_RE = re.compile(f"{DAMAGE_PHRASE_RE.pattern}(?:\\s+plus\\s+{DAMAGE_PHRASE_RE.pattern})*")
HIT_DAMAGE_RE = re.compile(r"\d+ (?:\(\d+d\d+(?:\s*[+-]\s*\d+)?\) )?[A-Za-z]+ damage")
ATTACK_RE = re.compile(r"(?:Melee|Ranged)(?: or (?:Melee|Ranged))? Attack Roll: \+(\d+)")
SAVE_RE = re.compile(r"(Strength|Dexterity|Constitution|Intelligence|Wisdom|Charisma) Saving Throw: DC (\d+)")
RECHARGE_RE = re.compile(r"Recharge (\d)(?:\s*[-–]\s*6)?")
PER_DAY_NAME_RE = re.compile(r"\((\d+)/Day\)")
SUBJECT_RE = re.compile(r"(?:^|[.:;]\s+|\bthen\s+|\band\s+)the ([a-z]+(?: [a-z]+)*)", re.I)
# Sentence subjects in SRD ability prose that are not creatures, from a survey of all 341 stat blocks.
NON_CREATURE_SUBJECTS = {"ally", "attack", "duration", "first", "heart", "new", "next",
                         "object", "original", "penalty", "puller", "spell", "stored", "target", "touch",
                         "wearer", "weapon", "wound"}
# Short self-references the printed stat block uses, checked against the 2024 Monster Manual.
SELF_REFERENCES = {"warhorse": {"horse"}}
# Capitalized rules terms that open a sentence ("The Poisoned condition", "The Cube is Heavily Obscured").
TERM_SUBJECTS = CONDITIONS | {"cone", "cube", "cylinder", "emanation", "line", "sphere"}
# Non-creature subjects that are also SRD creature names, allowed only where the prose means the object.
OBJECT_SUBJECTS = {"black-pudding": {"armor"}, "gray-ooze": {"armor"}}
CHOSEN_TYPE_RE = re.compile(r"\s*damage of the type chosen\b[^.]*")


SPELL_IDS = set(json.loads(SPELL_EFFECTS.read_text()))
SRD_SPELL_IDS = {spell["index"] for spell in json.loads(UPSTREAM_SPELLS.read_text())}


def singular(word):
    return word[:-1] if word.endswith("s") and not word.endswith("ss") else word


def words(text):
    return {singular(w) for w in re.findall(r"[a-z]+", text.lower())}


def slug(text):
    return re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-")


def assign_ids(names):
    seen, ids = set(), []
    for name in names:
        base = slug(name) or "ability"
        candidate, n = base, 2
        while candidate in seen:
            candidate = f"{base}-{n}"
            n += 1
        seen.add(candidate)
        ids.append(candidate)
    return ids


def expected_pb(cr):
    for limit, pb in PB_BANDS:
        if cr <= limit:
            return pb
    raise ValueError(f"no PB band for CR {cr}")


def hit_dice(monster):
    roll = re.sub(r"\s+", "", monster["hit_points_roll"])
    if not re.fullmatch(r"\d+d\d+(?:[+-]\d+)?", roll):
        raise ValueError(f"unexpected hit_points_roll {monster['hit_points_roll']!r} on {monster['index']}")
    return roll


def dice_text(dice, modifier):
    return f"{dice}{modifier:+d}" if modifier else dice


COOLDOWN_RE = re.compile(r"\bThe [a-z]+(?: [a-z]+)* can.t take this action again until the start of its next turn\.")
NO_PROVOKE = re.compile(r"^The [\w -]+ doesn.t provoke an Opportunity Attack when it (?:flies|moves) out of an enemy.s reach\.$")

class Review:
    def __init__(self):
        self.entries = []

    def add(self, monster, cls, path, summary, status="open"):
        self.entries.append({"monster": monster, "class": cls, "path": path,
                             "status": status, "summary": summary})


def hit_sentence(desc):
    idx = desc.find("Hit:")
    if idx < 0:
        return None, ""
    rest = desc[idx + 4:].lstrip()
    m = re.match(r"(.*?\.)(?=\s|$)", rest)
    if not m:
        return rest, ""
    return m.group(1), rest[m.end():].strip()


def hit_tail(sentence):
    """The effects a Hit sentence states after its damage, as an agent clause, or None.

    A sentence with damage keeps only a tail that opens with "and" (", and the target has ...").
    Any other tail is a conditional or alternative damage entry that `place_damage` already handles.
    A sentence with no damage at all is an effect in full (Roper's Tentacle).
    """
    text = sentence.replace("−", "-").replace("–", "-")
    start = 0
    for last in HIT_DAMAGE_RE.finditer(text):
        start = last.end()
    tail = text[start:]
    if start:
        lead = re.match(r"[\s,;]*and\b\s*", tail)
        if not lead:
            return None
        start += lead.end()
    clause = sentence[start:].strip(" ,;.")
    return f"{clause[:1].upper()}{clause[1:]}." if clause else None


class ActionIndex(set):
    """Action ids on one stat block, plus what a multiattack may name besides them."""

    def __init__(self, ids, raws):
        super().__init__(ids)
        self.aliases = {}
        for aid, raw in zip(ids, raws):
            bare = slug(re.sub(r"\s*\([^)]*\)\s*$", "", raw["name"]))
            if bare != aid and bare not in ids:
                self.aliases[bare] = aid
        self.spellcasting, self.spells = None, {}
        for aid, raw in zip(ids, raws):
            if "spellcasting" in raw and self.spellcasting is None:
                self.spellcasting = aid
                self.spells = {slug(s["name"]): s["index"] for s in raw["spellcasting"].get("spells", [])}


class Converter:
    def __init__(self, monster, review, all_names):
        self.monster = monster
        self.mid = monster["index"]
        self.review = review
        self.own_name = monster["name"].lower()
        self.own_words = words(monster["name"])
        self.flags = []
        self.other_names = sorted({n.lower() for n in all_names if n.lower() != self.own_name}, key=len, reverse=True)

    def self_reference(self, raw, base):
        """Flag prose whose sentence subject names a creature other than this one (class a).

        A subject "The X" passes when X is a word of this creature's name, a word of the
        ability's name ("The specter" in Create Specter), a non-creature subject such as
        "the target", or a short name in SELF_REFERENCES. A subject that names another
        upstream creature in full fails even when its words hide inside this name ("the
        mage" in the Archmage). A capitalized rules term ("The Poisoned condition", "The Cube")
        passes; a lowercase "the cube" can still mean the Gelatinous Cube.
        """
        desc = raw.get("desc", "")
        allowed = (self.own_words | words(raw["name"]) | NON_CREATURE_SUBJECTS
                   | SELF_REFERENCES.get(self.mid, set()) | OBJECT_SUBJECTS.get(self.mid, set()))
        for subject in SUBJECT_RE.finditer(desc):
            phrase = subject.group(1).lower()
            other = next((n for n in self.other_names if f"{phrase} ".startswith(f"{n} ")
                          and not words(n) <= self.own_words | words(raw["name"])), None)
            term = subject.group(1)[0].isupper() and phrase.split()[0] in TERM_SUBJECTS
            if other or not (term or singular(phrase.split()[0]) in allowed):
                named = other or phrase.split()[0]
                self.rec("a", f"{base}.desc", f"prose names 'the {named}' instead of this creature")
                return

    def rec(self, cls, path, summary):
        self.review.add(self.mid, cls, path, summary)

    def damage_list(self, raw_list, base):
        out = []
        for d in raw_list or []:
            m = DICE_RE.match(re.sub(r"\s+", "", d["damage_dice"]))
            dtype = d["damage_type"]["index"] if isinstance(d.get("damage_type"), dict) else None
            if not m or dtype not in DAMAGE_TYPES:
                self.rec("b", f"{base}.damage", f"damage '{d['damage_dice']}' {dtype} is not representable")
                return []
            mod = int(m.group(4) or 0) * (-1 if m.group(3) == "-" else 1)
            out.append({"dice": f"{m.group(1)}d{m.group(2)}", "type": dtype, "modifier": mod, "magical": False})
        return out

    def usage(self, raw, base):
        u = raw.get("usage")
        name_desc = f"{raw['name']} {raw.get('desc', '')}"
        if u and u["type"] == "recharge on roll":
            if u.get("dice") != "1d6":
                self.rec("b", f"{base}.usage", f"recharge dice {u.get('dice')} is not representable")
                return None
            minimum = u.get("min_value")
            if minimum not in (2, 3, 4, 5, 6):
                self.rec("b", f"{base}.usage", f"recharge {minimum}-6 has no engine range")
                return None
            usage = {"recharge": {"dice": f"{minimum}-6" if minimum < 6 else "6"}}
        elif u and u["type"] == "recharge after rest":
            usage = {"per_rest": {"rest": "short", "uses": 1}}
        elif u and u["type"] == "per day":
            usage = {"per_day": {"uses": u["times"]}}
        elif u:
            self.rec("b", f"{base}.usage", f"usage type '{u['type']}' is not representable")
            return None
        else:
            usage = None
        prose = RECHARGE_RE.search(name_desc)
        if prose:
            if usage is None or "recharge" not in usage:
                self.rec("b", f"{base}.usage", f"prose says Recharge {prose.group(1)}-6 but no recharge usage")
            elif int(prose.group(1)) != int(usage["recharge"]["dice"][0]):
                self.rec("a", f"{base}.usage", f"prose recharge {prose.group(1)} disagrees with structured usage")
        if COOLDOWN_RE.search(raw.get("desc", "")):
            if usage is None:
                usage = "cooldown_until_next_turn"
                self.rec("d", f"{base}.usage", "set to cooldown_until_next_turn from the prose")
            else:
                self.rec("a", f"{base}.usage", "prose says the action waits until the next turn, structured usage differs")
        per_day = PER_DAY_NAME_RE.search(raw["name"])
        if per_day:
            if usage is None or "per_day" not in usage:
                self.rec("b", f"{base}.usage", f"name says {per_day.group(1)}/Day but no per-day usage")
            elif usage["per_day"]["uses"] != int(per_day.group(1)):
                self.rec("a", f"{base}.usage", f"name says {per_day.group(1)}/Day, structured usage differs")
        return usage

    def place_damage(self, text, limit, damage, base, label):
        """Sort structured damage entries by where the prose states them.

        `text` is the effect's prose and `limit` the length of its first sentence. An entry stated
        unconditionally in the first sentence stays in `damage`. A conditional entry is gated on the
        target's Grappled condition (from this creature) when the prose says so, and otherwise leaves
        the model as an agent clause. An entry stated in a later sentence is recurring or conditional
        in every SRD case, so it leaves the model too; the caller keeps the sentence as a clause. The
        exception is an "Otherwise" sentence, which states the immediate damage after an HP gate.
        Returns the kept entries and the clauses for entries taken out of `damage`.
        """
        text = text.replace("−", "-").replace("–", "-")
        spots = [(m.start(), m.end(), dice_text(m.group(1), int(m.group(3) or 0) * (-1 if m.group(2) == "-" else 1)))
                 for m in PROSE_DICE_RE.finditer(text)]
        paired = {}
        for index, entry in enumerate(damage):
            want = dice_text(entry["dice"], entry["modifier"])
            spot = next((i for i, s in enumerate(spots) if i not in paired.values() and s[2] == want), None)
            if spot is None:
                self.rec("a", f"{base}.damage", f"structured damage {want} {entry['type']} is not stated in the prose")
            else:
                paired[index] = spot
        kept, clauses = [], []
        for i, (start, end, dice) in enumerate(spots):
            if start < limit and i not in paired.values() and re.match(r"\)?\s*(?:[A-Za-z]+\s+)?damage", text[end - 1:]):
                chosen = CHOSEN_TYPE_RE.match(text, end)
                if chosen:
                    lead = re.search(r"(?:\bplus\s+)?\d+\s*$", text[:start])
                    stated = text[lead.start() if lead else start:chosen.end()].strip()
                    clauses.append(f"Damage of a chosen type, not modeled: {stated}.")
                    self.rec("d", f"{base}.damage", f"{label} dice {dice} have a chosen damage type; kept as an agent clause")
                else:
                    self.rec("b", f"{base}.damage", f"{label} dice {dice} are not in the structured damage")
        for index, entry in enumerate(damage):
            if index not in paired:
                kept.append(entry)
                continue
            start, end, _ = spots[paired[index]]
            region = limit
            if start >= limit:
                sentence = text[text.rfind(".", 0, start) + 1:max(text.find(".", end), end)]
                if not OTHERWISE_RE.search(sentence) or RECURRING_RE.search(sentence):
                    continue
                region = len(text)
            prose_type = re.match(r"\s*([A-Za-z]+) damage", text[end:])
            if prose_type and prose_type.group(1).lower() != entry["type"]:
                self.rec("a", f"{base}.damage", f"{label} type {prose_type.group(1).lower()} vs structured {entry['type']}")
            previous = max((s[1] for s in spots[:paired[index]]), default=0)
            following = next((s[0] for s in spots[paired[index] + 1:] if s[0] < region), region)
            trail = text[end:following]
            if not DAMAGE_IF_RE.match(trail):
                kept.append(entry)
                continue
            alternative = re.search(r"\bor\s+\d+\s*$", text[previous:start])
            if GRAPPLED_BY_SELF_RE.search(trail):
                gate = {"if_target": "grappled", "if_target_from_actor": True}
                if not alternative:
                    kept.append({**entry, **gate})
                    continue
                extra = self.extra_dice(kept[-1] if kept else None, entry)
                if extra:
                    kept.append({**extra, **gate})
                    continue
            lead = re.search(r"(?:(?:—|,)?\s*\b(?:or|plus)\s+)?\d+\s*$", text[:start])
            stated = text[start - len(lead.group(0)) if lead else start:end + len(re.split(r"—|,?\s+plus\s", trail)[0])]
            clauses.append(f"Conditional damage, not modeled: {stated.strip(' ,—')}")
        return kept, clauses

    @staticmethod
    def extra_dice(base_entry, alternative):
        """The dice an alternative adds to a base entry of the same die, type and modifier, or None."""
        if base_entry is None or base_entry["type"] != alternative["type"] or base_entry["modifier"] != alternative["modifier"]:
            return None
        have, want = DICE_RE.match(base_entry["dice"]), DICE_RE.match(alternative["dice"])
        if not have or not want or have.group(2) != want.group(2) or int(want.group(1)) <= int(have.group(1)):
            return None
        return {"dice": f"{int(want.group(1)) - int(have.group(1))}d{want.group(2)}", "type": alternative["type"],
                "modifier": 0, "magical": False}

    def attack(self, raw, base):
        desc = raw.get("desc", "")
        prose = ATTACK_RE.search(desc)
        if prose and int(prose.group(1)) != raw["attack_bonus"]:
            self.rec("a", f"{base}.effects", f"prose attack +{prose.group(1)} vs structured {raw['attack_bonus']}")
        reach = re.search(r"reach (\d+) (?:ft|feet)", desc)
        rng = re.search(r"range (\d+)/(\d+) (?:ft|feet)", desc) or re.search(r"range (\d+) (?:ft|feet)", desc)
        if rng and rng.lastindex == 1:
            rng_value = [int(rng.group(1)), int(rng.group(1))]
        else:
            rng_value = [int(rng.group(1)), int(rng.group(2))] if rng else None
        if ATTACK_RANGED_RE.search(desc) and rng is None:
            self.rec("b", f"{base}.effects", "ranged attack without a stated range")
        damage = self.damage_list(raw.get("damage"), base)
        clauses = []
        sentence, rest = hit_sentence(desc)
        if sentence is not None:
            if not damage and PROSE_DICE_RE.search(sentence):
                self.rec("b", f"{base}.damage", "Hit clause states damage but structured damage is missing")
            elif not damage and "damage" in sentence.lower():
                self.rec("b", f"{base}.damage", "Hit clause states flat damage but structured damage is missing")
            else:
                damage, extra = self.place_damage(f"{sentence} {rest}", len(sentence), damage, base, "Hit clause")
                clauses.extend(extra)
                for flat in FLAT_DAMAGE_RE.finditer(sentence):
                    if not any(d["dice"] == f"{flat.group(1)}d1" and d["type"] == flat.group(2).lower() for d in damage):
                        self.rec("b", f"{base}.damage", "Hit clause states flat damage but structured damage is missing")
            tail = hit_tail(sentence)
            if tail:
                clauses.append(tail)
            if rest:
                clauses.append(rest)
        effect = {"attack": {
            "modifier": raw["attack_bonus"],
            "reach": int(reach.group(1)) if reach else None,
            "range": rng_value,
            "target_count": 1,
            "damage": damage,
            "on_hit": [],
        }}
        return effect, clauses

    def saving_throw(self, raw, base):
        desc = raw.get("desc", "")
        dc = raw["dc"]
        ability = dc["dc_type"]["index"]
        if ability not in ABILITY_KEYS:
            self.rec("b", f"{base}.effects", f"save ability '{ability}' is not representable")
            return None, []
        prose = SAVE_RE.search(desc)
        if prose and (ABILITY_NAMES[prose.group(1)] != ability or int(prose.group(2)) != dc["dc_value"]):
            self.rec("a", f"{base}.effects", f"prose save {prose.group(1)} DC {prose.group(2)} vs structured {ability} DC {dc['dc_value']}")
        success = dc["success_type"]
        success_text = re.search(r"Success:\s*(.*?)(?=\s*Failure:|$)", desc)
        if success_text and ("half damage" in success_text.group(1).lower()) != (success == "half"):
            self.rec("a", f"{base}.effects", f"prose success clause vs structured success_type '{success}'")
        damage = self.damage_list(raw.get("damage"), base)
        failure = re.search(r"Failure:\s*(.*?)(?=\s*Success:|$)", desc)
        if failure is None and damage:
            self.rec("d", f"{base}.damage", "dc without a Failure clause is an ability check, not a saving throw; left unmodeled")
            return None, []
        failure_text = failure.group(1) if failure else ""
        if DAMAGE_PHRASE_RE.search(failure_text) and not damage:
            self.rec("b", f"{base}.damage", "Failure clause states damage but structured damage is missing")
        if not damage:
            return None, []
        first = re.match(r"[^.]*\.?", failure_text).end()
        damage, clauses = self.place_damage(failure_text, first, damage, base, "Failure clause")
        if not damage:
            self.rec("d", f"{base}.damage", "Failure clause states damage only as recurring or conditional later clauses; left unmodeled")
            return None, []
        later = "".join(
            DAMAGE_CHAIN_RE.sub("the modeled damage", sentence)
            if OTHERWISE_RE.search(sentence) and not RECURRING_RE.search(sentence) else sentence
            for sentence in re.split(r"(?<=\.)(\s+)", failure_text[first:]))
        remainder = (DAMAGE_CHAIN_RE.sub("", failure_text[:first]) + later).strip(" .,;")
        if remainder:
            clauses.append(remainder)
        effect = {"saving_throw": {
            "ability": ability,
            "dc": dc["dc_value"],
            "half_on_success": success == "half",
            "no_damage_on_success": success == "none",
            "repeat_save": None,
            "two_stage": None,
            "immunity_after_success": False,
            "hp_gate": None,
            "creature_type_gate": None,
            "damage": damage,
            "on_failure": [],
        }}
        return effect, clauses

    def slot(self, name, desc, action_ids):
        """The multiattack slot a named action or Spellcasting spell resolves to, or None.

        An action named with a trailing parenthetical ("Entangling Rope (Requires Magic Rope)")
        also answers to its bare name. A spell listed in the Spellcasting action becomes a cast
        through that action, keeping a "(level N version)" note from the prose as an upcast.
        """
        target = slug(name)
        if target in action_ids:
            return {"action_id": target}
        if target in action_ids.aliases:
            return {"action_id": action_ids.aliases[target]}
        spell = action_ids.spells.get(target)
        if spell is None:
            return None
        level = re.search(rf"{re.escape(name)}\s*\(level (\d+) version\)", desc)
        if spell not in SRD_SPELL_IDS:
            self.rec("b", f"actions.{action_ids.spellcasting}", f"multiattack casts '{spell}', which is not an SRD 5.2.1 spell")
        elif spell not in SPELL_IDS:
            self.rec("d", f"actions.{action_ids.spellcasting}", f"multiattack casts '{spell}', which data/spells/effects.json lacks; the engine logs it as unmodeled")
        return {"action_id": action_ids.spellcasting, "cast": {
            "spell_id": spell, "slot_level": int(level.group(1)) if level else None, "upcast": bool(level)}}

    def action_step(self, act, desc, action_ids, missing):
        found = self.slot(act["action_name"], desc, action_ids)
        if found is None:
            missing.append(slug(act["action_name"]))
            return None
        return {"action": {**found, "count": act["count"]}}

    def multiattack(self, raw, base, action_ids):
        steps, ok = [], True
        desc = raw.get("desc", "")
        opts = raw.get("action_options")
        options = (opts or {}).get("from", {}).get("options", [])
        counts = [a.get("count") for a in raw.get("actions") or []]
        counts += [o.get("count") for o in options if o.get("option_type") != "multiple"]
        counts += [i.get("count") for o in options if o.get("option_type") == "multiple" for i in o.get("items", [])]
        if any(not isinstance(c, int) for c in counts):
            self.rec("b", f"{base}.effects", "multiattack count is variable and not representable")
            return []
        for act in raw.get("actions") or []:
            step = self.action_step(act, desc, action_ids, [])
            if step is None:
                self.rec("a", f"{base}.effects", f"multiattack names '{act['action_name']}' with no matching action")
                ok = False
                continue
            steps.append(step)
        if opts:
            from_ = opts.get("from", {})
            choose = opts.get("choose", 1)
            grouped = any(o.get("option_type") == "multiple" for o in options)
            representable = (from_.get("option_set_type") == "options_array"
                             and options
                             and all(o.get("option_type") in ("action", "multiple") for o in options)
                             and all(i.get("option_type") == "action" for o in options for i in o.get("items", []))
                             and (choose == 1 if grouped else choose in (1, 2, 3)))
            missing = []
            if not representable:
                reason = "unsupported option set"
                if any(o.get("option_type") not in ("action", "multiple") for o in options):
                    reason = "nested option group"
                elif choose not in (1, 2, 3) or grouped:
                    reason = f"choose-{choose} combination"
                self.rec("b", f"{base}.effects", f"multiattack {reason} is not representable in the closed vocabulary")
                ok = False
            elif grouped:
                # "Two Claw attacks and one Snake Hair attack, or three Poison Ray attacks":
                # each upstream option is one alternative set of fixed steps.
                sets = [[self.action_step(i, desc, action_ids, missing) for i in (o["items"] if o["option_type"] == "multiple" else [o])]
                        for o in options]
                if not missing:
                    steps.append({"alternatives": {"sets": sets}})
            else:
                # Expand each upstream option into `count` distinct pick slots.
                # This lets a `Choice { options, pick = choose }` represent SRD
                # "any combination" patterns where the same action can be chosen
                # multiple times. The engine resolves `pick` distinct slots.
                choice_options = []
                for o in options:
                    found = self.slot(o["action_name"], desc, action_ids)
                    if found is None:
                        missing.append(slug(o["action_name"]))
                        continue
                    choice_options += [{**found, "count": 1} for _ in range(o.get("count", 1))]
                if not missing:
                    steps.append({"choice": {"options": choice_options, "pick": choose}})
            if missing:
                self.rec("a", f"{base}.effects", f"multiattack options name missing actions {missing}")
                ok = False
        if not steps and ok:
            self.rec("b", f"{base}.effects", "multiattack has no representable steps")
            ok = False
        if not ok:
            return []
        return [{"multiattack": {"steps": steps}}]

    def ability(self, raw, section, aid, action_ids):
        base = f"{section}.{aid}"
        desc = raw.get("desc", "")
        self.self_reference(raw, base)
        entry = {"id": aid, "name": raw["name"], "desc": desc, "kind": "unmodeled",
                 "default_resolution": "skipped"}
        effects, clauses = [], []
        if raw.get("multiattack_type"):
            effects = self.multiattack(raw, base, action_ids)
        elif raw.get("attack_bonus") is not None:
            effect, clauses = self.attack(raw, base)
            effects = [effect]
        elif raw.get("dc"):
            effect, clauses = self.saving_throw(raw, base)
            effects = [effect] if effect else []
        if section == "traits" and NO_PROVOKE.search(desc):
            effects = ["no_provoke"]
        usage = self.usage(raw, base)
        if usage:
            entry["usage"] = usage
        if effects:
            entry.pop("default_resolution")
            entry["effects"] = effects
            entry["kind"] = "partly_modeled" if clauses else "modeled"
            if clauses:
                entry["agent_clauses"] = clauses
        if "spellcasting" in raw:
            self.rec("d", base, "spellcasting is not modeled; spells are absent from the stat block effects")
        if not effects and raw.get("damage"):
            self.rec("d", base, "damage array without an attack or save; left unmodeled")
        self.rec("d", base, f"classified {entry['kind']}" +
                 (f" (default {entry['default_resolution']})" if "default_resolution" in entry else ""))
        return entry

    def sections(self):
        raw_sections = {sec: (self.mid_raw(key) or []) for sec, key in SECTIONS.items()}
        ids = {sec: assign_ids([a["name"] for a in raw_sections[sec]]) for sec in SECTIONS}
        action_ids = ActionIndex(ids["actions"], raw_sections["actions"])
        return {sec: [self.ability(raw, sec, aid, action_ids)
                      for raw, aid in zip(raw_sections[sec], ids[sec])]
                for sec in SECTIONS}

    def mid_raw(self, key):
        return self.monster.get(key)

    def damage_types(self, values, key):
        types, leftovers = [], []
        for value in values:
            text = value.lower() if isinstance(value, str) else value.get("name", "").lower()
            for token in re.split(r",|;| and | or ", text):
                token = token.strip()
                if not token:
                    continue
                if token in DAMAGE_TYPES:
                    types.append(token)
                else:
                    leftovers.append(token)
        unknown = [t for t in leftovers if "damage" not in t]
        if unknown:
            self.rec("b", key, f"qualifier not representable: {unknown}")
        for qualifier in (t for t in leftovers if "damage" in t):
            self.flags.append({"path": key, "note": f"Not modeled; the agent applies it: {qualifier}"})
            self.rec("d", key, f"qualifier kept as an intake flag: {qualifier}")
        return types

    def convert(self):
        m = self.monster
        raw_size = m["size"].lower()
        size_choices = [s.strip() for s in raw_size.split(" or ")]
        size = size_choices[0]
        if size not in SIZES:
            self.rec("b", "size", f"size '{m['size']}' is not representable")
        for s in size_choices:
            if s not in SIZES:
                self.rec("b", "size", f"size choice '{s}' is not representable")
        block = {
            "id": self.mid,
            "name": m["name"],
            "source": SOURCE,
            "schema_version": SCHEMA_VERSION,
            "size": size,
            **({"size_choice": size_choices} if len(size_choices) > 1 else {}),
            "type": m["type"],
            "alignment": m["alignment"],
            "armor_class": [],
            "hit_points": {"average": m["hit_points"], "dice": hit_dice(m)},
            "speeds": {},
            "ability_scores": {ABILITY_KEYS[k]: m[ABILITY_KEYS[k]] for k in ABILITY_KEYS},
            "initiative": (m["dexterity"] - 10) // 2,
            "proficiency_bonus": m["proficiency_bonus"],
        }
        for ac in m["armor_class"]:
            source = ", ".join(a["name"] if isinstance(a, dict) else str(a) for a in ac["armor"]) if ac.get("armor") else None
            block["armor_class"].append({"value": ac["value"], **({"source": source} if source else {})})
        speed = m["speed"]
        for key, val in speed.items():
            if key == "hover":
                continue
            found = re.fullmatch(r"(\d+) ft\.", val)
            if not found:
                self.rec("b", "speeds", f"speed '{key}: {val}' is not representable")
                continue
            block["speeds"][key] = {"feet": int(found.group(1))}
        if speed.get("hover") is True:
            if "fly" in block["speeds"]:
                block["speeds"]["fly"]["hover"] = True
            else:
                self.rec("b", "speeds", "hover without a fly speed is not representable")
        saves, skills = {}, {}
        for prof in m.get("proficiencies", []):
            key = prof["proficiency"]["index"]
            if key.startswith("saving-throw-"):
                saves[key.removeprefix("saving-throw-")] = prof["value"]
            elif key.startswith("skill-"):
                skills[key.removeprefix("skill-")] = prof["value"]
        if saves:
            block["saving_throws"] = saves
        if skills:
            block["skills"] = skills
        for key, output in (("damage_vulnerabilities", "damage_vulnerabilities"),
                            ("damage_resistances", "damage_resistances"),
                            ("damage_immunities", "damage_immunities")):
            found = self.damage_types(m[key], key)
            if found:
                block[output] = found
        conditions = [c["name"].lower() for c in m["condition_immunities"]]
        bad = [c for c in conditions if c not in CONDITIONS]
        if bad:
            self.rec("b", "condition_immunities", f"unknown conditions {bad}")
        if conditions:
            block["condition_immunities"] = [c.replace(" ", "_") for c in conditions if c in CONDITIONS]
        senses = {}
        for key in ("darkvision", "blindsight", "tremorsense", "truesight"):
            if key in m["senses"]:
                senses[key] = int(re.match(r"(\d+)", m["senses"][key]).group(1))
        senses["passive_perception"] = m["senses"]["passive_perception"]
        block["senses"] = senses
        languages = [] if m["languages"].strip().lower() == "none" else \
            [p.strip() for p in re.split(r";|,", m["languages"]) if p.strip()]
        if languages:
            block["languages"] = languages
        cr = m["challenge_rating"]
        block["challenge_rating"] = cr
        block["xp"] = m["xp"]
        if "xp_in_lair" in m:
            block["lair_xp"] = m["xp_in_lair"]
        if cr in XP_BY_CR and m["xp"] not in XP_BY_CR[cr]:
            self.rec("c", "xp", f"xp {m['xp']} vs SRD table {sorted(XP_BY_CR[cr])} at CR {cr}")
        if m["proficiency_bonus"] != expected_pb(cr):
            self.rec("c", "proficiency_bonus", f"PB {m['proficiency_bonus']} vs SRD table {expected_pb(cr)} at CR {cr}")
        sections = self.sections()
        for sec in SECTIONS:
            if sections[sec]:
                block[sec] = sections[sec]
        if self.flags:
            block["intake_flags"] = self.flags
        if m.get("legendary_actions"):
            block["legendary_uses_per_round"] = LEGENDARY_USES_PER_ROUND
            self.rec("d", "legendary_uses_per_round", f"set to SRD default {LEGENDARY_USES_PER_ROUND}; upstream omits the count")
        return block


def apply_path(blocks, monster_id, path, value, review_fail):
    block = blocks[monster_id]
    parts = path.split(".")
    if len(parts) == 1:
        if path == "xp":
            block["xp"] = value
        elif path in SECTIONS:
            block[path] = value
        else:
            review_fail(f"correction path '{path}' on {monster_id} matches nothing")
        return
    if len(parts) == 3:
        section, aid, field = parts
        ability = next((a for a in block.get(section, []) if a["id"] == aid), None)
        if ability is None:
            review_fail(f"correction {monster_id}.{path} matches no upstream ability")
            return
        if field == "usage":
            ability["usage"] = value
            return
        if field == "desc":
            ability["desc"] = value
            return
        if field in ("effects", "agent_clauses"):
            ability[field] = value
            classify(ability)
            return
        if field == "damage":
            for effect in ability.get("effects", []):
                for key in ("attack", "saving_throw"):
                    if key in effect:
                        effect[key]["damage"] = value
                        return
            review_fail(f"correction {monster_id}.{path} has no attack or save to hold damage")
            return
    review_fail(f"correction path '{path}' on {monster_id} is not supported")


def classify(ability):
    """Reset an ability's kind after a correction replaces its effects or agent clauses."""
    if ability.get("effects"):
        ability.pop("default_resolution", None)
        ability["kind"] = "partly_modeled" if ability.get("agent_clauses") else "modeled"
    else:
        ability["kind"] = "unmodeled"
        ability.setdefault("default_resolution", "skipped")


def covers(correction_path, mismatch_path):
    return mismatch_path == correction_path or mismatch_path.startswith(correction_path + ".")


def main():
    commit = COMMIT_FILE.read_text().strip()
    upstream = json.loads(UPSTREAM.read_text())
    corrections = json.loads(CORRECTIONS.read_text())
    all_names = [m["name"] for m in upstream]
    review = Review()

    blocks = {}
    for monster in upstream:
        blocks[monster["index"]] = Converter(monster, review, all_names).convert()

    failures = []
    for corr in corrections:
        if corr["commit"] != commit:
            failures.append(f"correction for {corr['monster_id']} written against {corr['commit']}, not {commit}")
            continue
        label = f"{corr['monster_id']}.{corr['path']}"
        if corr["monster_id"] not in blocks:
            failures.append(f"correction {label} matches no upstream monster")
            continue
        if corr.get("applies") is False:
            review.add(corr["monster_id"], "b", corr["path"],
                       f"correction not applied: {corr['reason']}")
            continue
        before = len(failures)
        apply_path(blocks, corr["monster_id"], corr["path"], corr["value"], failures.append)
        if len(failures) == before:
            for entry in review.entries:
                if entry["monster"] == corr["monster_id"] and entry["status"] == "open" \
                        and entry["class"] in ("a", "b", "c") and covers(corr["path"], entry["path"]):
                    entry["status"] = "corrected"
                    entry["summary"] += f" (corrected: {corr['reason']})"
            review.add(corr["monster_id"], "d", corr["path"],
                       f"correction applied: {corr['reason']} [{corr['citation']}]")
    if failures:
        print("Import aborted:", file=sys.stderr)
        for line in failures:
            print(f"  {line}", file=sys.stderr)
        sys.exit(2)

    graduated, blocked = [], []
    open_blocking = {}
    for entry in review.entries:
        if entry["status"] == "open" and entry["class"] in ("a", "b", "c"):
            open_blocking.setdefault(entry["monster"], []).append(entry)
    FIXTURES.mkdir(parents=True, exist_ok=True)
    for stale in FIXTURES.glob("*.json"):
        stale.unlink()
    for monster_id in sorted(blocks):
        if monster_id in open_blocking:
            blocked.append(monster_id)
            continue
        graduated.append(monster_id)
        (FIXTURES / f"{monster_id}.json").write_text(
            json.dumps(blocks[monster_id], indent=2, ensure_ascii=False, sort_keys=True) + "\n")

    review.entries.sort(key=lambda e: (e["monster"], e["class"], e["path"], e["summary"]))
    open_count = sum(len(v) for v in open_blocking.values())
    summary = {"commit": commit, "monsters": len(blocks), "graduated": len(graduated),
               "blocked": len(blocked), "open_blocking": open_count,
               "corrections": len(corrections)}
    REVIEW_JSON.write_text(json.dumps({"summary": summary, "entries": review.entries},
                                      indent=2, ensure_ascii=False) + "\n")
    REVIEW_MD.write_text(render_markdown(summary, review.entries, blocked))

    print(f"SRD import at {commit}: {summary['graduated']} of {summary['monsters']} monsters graduated, "
          f"{summary['blocked']} blocked, {summary['open_blocking']} open blocking entries.")
    if open_count:
        sys.exit(1)


def render_markdown(summary, entries, blocked):
    lines = [
        "# SRD import review",
        "",
        "Generated by `scripts/import-srd-monsters.py`. Do not edit by hand; regenerate on each run.",
        "",
        f"- Commit: `{summary['commit']}`",
        f"- Monsters: {summary['monsters']}; graduated to `data/fixtures/`: {summary['graduated']}; blocked: {summary['blocked']}",
        f"- Open blocking entries (classes a to c): {summary['open_blocking']}",
        f"- Corrections in `data/srd/corrections.json`: {summary['corrections']}",
        "",
        "## Open blocking entries",
        "",
    ]
    open_blocking = [e for e in entries if e["status"] == "open" and e["class"] in ("a", "b", "c")]
    if not open_blocking:
        lines.append("None.")
    for e in open_blocking:
        lines.append(f"- `{e['monster']}` class {e['class']} `{e['path']}`: {e['summary']}")
    lines += ["", "## Corrected entries", ""]
    corrected = [e for e in entries if e["status"] == "corrected"]
    if not corrected:
        lines.append("None.")
    for e in corrected:
        lines.append(f"- `{e['monster']}` class {e['class']} `{e['path']}`: {e['summary']}")
    lines += ["", "## Notes (informational, never blocking)", ""]
    for e in entries:
        if e["class"] == "d":
            lines.append(f"- `{e['monster']}` `{e['path']}`: {e['summary']}")
    return "\n".join(lines) + "\n"


if __name__ == "__main__":
    main()
