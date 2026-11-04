---
id: 050
title: Upcast spell scaling
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

How does the spell effects file state what changes per slot level above a spell's base level, and how does `resolve_cast` apply it? Decide the field shape (extra damage dice, extra targets, longer duration), whether scaling is scaffolded from the SRD's higher-level text by `scripts/scaffold-spells.py`, and how a cast with no scaling entry is reported.

## Notes

`resolve_cast` ignores `Cast.slot_level` and `upcast`, so a dragon's level 3 Acid Arrow resolves at the base level. ADR 0014 governs the effects file and its honesty checks. Cantrip scaling by character level may share the answer. Consult `grilling` and `domain-modeling`.

## Resolution

Decided on 2026-11-04 by a delegated agent, under the user's instruction to make the decision without coming back. Evidence: ADR 0014, `bench-core/src/spell.rs` and `engine.rs`, `data/spells/effects.json`, the fixtures, PHB "Using a Higher-Level Spell Slot" (ch. 7, p. 234) and MM "Spellcasting" (How to Use a Monster, p. 11).

What the code does today: `SpellEffect.scaling` already exists, and 31 entries carry rules, but the engine never reads them. Several of those rules are wrong: Magic Missile adds a die to each of its three darts, Scorching Ray adds a target to each ray, and Acid Arrow, Vitriolic Sphere, Bless, Wall of Fire and Wall of Ice point at effects that don't exist. The scaffold writes rule shapes (`"extra_dice": "1d4"`) that don't match the schema. Cantrips are spotted by `slot_level == 0`, so a leveled spell with no slot level counts as a cantrip. Magic Missile (`1d4+1`), Heal (`70`), Disintegrate (`10d6+40`), Finger of Death (`7d8+30`) and Regenerate (`4d8+15`) hold dice strings `roll_dice` can't parse. Standard casters cast Disintegrate at levels 11 to 20.

- **Cast level.** A cast resolves at `slot_level`, or at the spell's own level when `slot_level` is empty. A slot level below the spell's level, or any slot level on a cantrip, is refused. Reason: the PHB says the spell takes on the slot's level, and the MM says a monster casts at the lowest level unless its stat block says otherwise ("level N version").
- **Spell level in the file.** Each effects entry gains a required `level`, written by the scaffold from the SRD and checked against it. Reason: the engine needs the base level to count steps, and only the effects file is embedded.
- **`Cast.upcast`.** It stays as the importer's record of a "level N version" note. The engine ignores it. A test asserts that every fixture cast has `upcast` exactly when `slot_level` is above the spell's level. Reason: removing it touches every fixture and the schema for no gain.
- **Rule shape.** `scaling` stays a list of `{effect_index, per_slot}`, applied once per level above the base. `per_slot` has four variants. `extra_dice {dice, damage_type?}` adds to the matching damage entry, or the first one, or to Heal or TempHp dice, looking inside Area. The die size must match, so `4d4` plus one `1d4` is `5d4`. `extra_modifier {amount}` adds a flat amount to Heal, TempHp or Damage. `extra_targets {count}` raises `target_count` on Attack or Area. On other effects it changes nothing, because they already take every target passed. Hold Person keeps its rule as a record of the rules text. `extra_effects {count}` appends copies of the effect, for darts and rays. Reason: this covers every damage and healing upcast in the file without changing the dice grammar.
- **Flat healing.** `Heal.dice` becomes optional, and a Heal can turn off the spellcasting modifier. The Heal spell is a flat 70 plus 10 per level. Reason: `bind_effect` adds the caster's modifier to every Heal, and a `1d1` workaround would ask a live player to roll a d1.
- **Retired variants.** `increase_duration` and `add_damage_type` go. Reason: no entry uses them, and SRD duration upcasts run minutes to days, longer than a fight.
- **What the file can't express.** An entry may carry `upcast_clause`, the text of what a higher slot does that the engine doesn't apply. An entry can have both rules and a clause. Examples: Acid Arrow's later damage, Arcane Hand's Grasping Hand, Ice Knife's cold burst, Aid, Dominate durations. Reason: this matches the agent-clause pattern in ADR 0005.
- **Reporting a cast with no scaling.** An upcast of an entry with an `upcast_clause` logs an `upcast_clause` event that names the cast level and gives the text. A batch report warns once per such cast in the monster's stat block. An upcast of an entry with no rule and no clause resolves at the base level without comment, because the honesty check makes sure no rule is missing. Reason: an event the agent can act on, and a report line, without noise on spells that have no upcast at all.
- **Scaffold.** `scaffold-spells.py` reads the higher-level text from `higher_level` and from the description's "Using a Higher-Level Spell Slot." paragraph. It writes schema-valid rules for the four patterns and an `upcast_clause` for anything else. Reason: ADR 0014 already chose scaffold-then-hand-finish, and the 2024 data keeps the text in either place.
- **Honesty check.** `check-spell-effects.py` requires `level` to match the SRD, every spell with higher-level text to have a rule or a clause, and each rule's dice or amount to appear in the text. A Rust test checks that each rule's `effect_index` exists, that the variant fits that effect, that the die sizes match, and that every dice string in the file parses. Reason: the faults above went unnoticed because nothing checked them.
- **Cantrips.** A cantrip is a spell with `level` 0. It keeps the existing multiplier from the caster's `level` at 5, 11 and 17, and it is not written as a scaling rule. A monster's cantrip stays at base dice. Reason: every damage cantrip's upgrade in the SRD is the same step on its dice, and the MM gives monsters no character level.
- **Live slot choice.** `bench action` gains `--slot <level>` for a combatant whose stat block has `spell_slots`. It sets the cast level of every Cast in the action. It is refused for a stat block with no `spell_slots`, an action with no leveled Cast, a level below the spell's, or no slot left. Reason: the user plays the party and chooses slots at the table, and the MM forbids monsters from upcasting.
- **Batch.** Batch passes no `--slot`. The tactics policy scores the scaled spell through `bound_cast_effects`. `POLICY_VERSION` stays at 2. Reason: the policy's rules don't change, only the effects it reads.
- **Versions and baseline.** `ENGINE_VERSION` goes to 0.1.2. Fourteen dragon fixtures cast "level N version" spells, so their batch numbers change. Fixing the five bad dice strings may change caster and healer cells. Moving upcast text out of `agent_clauses` changes 20 standard party files, so the party is regenerated and the curve also reports "the party differs". The curve stays stale until ticket 056 regenerates it, and 056 now waits on the build. Reason: ticket 056 owns the one regeneration.

Left out:

- The batch policy never chooses a higher slot. Phase 1 casts at the slot the stat block names. The standard party tables can name a higher slot if a later decision wants it.
- No target cap is added to Saving Throw, Condition or Heal effects. In live mode the agent passes the extra targets for spells like Hold Person. Batch picks one target for these, so their extra targets don't count there.
- Spells outside the effects file are unchanged and still log `unmodeled_cast`.

Build ticket: [Build upcast spell scaling](../tickets/059-build-upcast-spell-scaling.md).
