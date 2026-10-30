---
id: 037
title: Conditional and later-clause attack damage
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Add `Damage.if_target: Option<Condition>` with engine support, route later-clause unconditional damage to `Attack.on_hit`, update the importer for multi-damage and conditional damage, resolve the 'grappled by this creature' source-tracking question, review the 16 class-a Hit-clause entries, and verify the ADR 0005 wording matches what is built.

## Notes

Consult `implement`, `tdd` and `domain-modeling`. Decision recorded in [closed/035](../closed/035-srd-importer-vocabulary-gaps.md). Unblocks monsters in the ticket 028 importer; rerun `python3 -I scripts/import-srd-monsters.py` and `cargo test -p bench-core` afterward and report the new graduated count.

## Resolution

Built on branch `ticket-037-conditional-and-later-clause-damage`. SRD fixtures rose from 227 to 269 of 341, and `cargo test --all` passes (44 lib tests).

- **Engine.** `Damage` gains `if_target: Option<Condition>` and `if_target_from_actor: bool`. `damage_applies` in `bench-core/src/engine.rs` skips a gated entry in the attack, `Effect::Damage` and saving-throw paths. Resistance now applies to each damage entry instead of the running total, and half damage on a successful save respects resistance.
- **Grappled by this creature.** No new source tracking was needed: `ConditionInstance.source` already records who applied a condition, and `if_target_from_actor` compares it with the damaging creature. The only SRD user is the Mimic's Bite, where the importer turns "12 (2d8 + 3) … if Grappled by the mimic" into the base 1d8 + 3 plus a gated extra 1d8. The gate is inert in play: the engine only adds a condition through `Effect::Condition`, the importer emits no `on_hit` effects, and the Pseudopod grapple stays an agent clause. Modeling it needs a size limit on `ConditionEffect` and a way for a grapple to end.
- **Later-clause damage.** No SRD attack states plain unconditional damage after the Hit sentence, so the importer never routes damage to `Attack.on_hit`. Later-sentence damage is always conditional or recurring (a charge bonus, start-of-turn damage, swallowed-creature acid, Hit Point maximum loss), and it leaves the model as an agent clause. An ability whose only damage is recurring is left unmodeled. Six abilities (aboleth, behir, djinni, purple worm, tarrasque, vrock) used to graduate with that recurring damage modeled as an immediate hit and now stay unmodeled. An "Otherwise" sentence states immediate damage after an HP gate (Incubus, Sea Hag, Solar), so that damage stays on the failed save and the clause says "the modeled damage".
- **Conditions that are not target conditions.** Bloodied, Advantage and "moved 20+ feet" damage stays an agent clause with the base damage modeled.
- **The class-a Hit-clause entries.** Twelve actions list extra charge damage in the structured array (boar, elk, giant boar, giant elk, giant goat, both minotaurs, rhinoceros, triceratops, warhorse, two wereboar forms) and three list recurring damage (animated rug, pit fiend, stirge). The prose states all of it conditionally or per turn, so none is an upstream fault. The importer now pairs each structured entry with the prose that states it, and these entries no longer mismatch. The remaining class-a entries are multiattack names that miss spell actions (dragons, cloud giant, dryad, erinyes), which belong to other tickets.
- **Flat 1 damage.** A new check for flat damage in the Hit sentence found five real upstream omissions, now in `data/srd/corrections.json` (goat, cultist, flying-snake, scorpion, spider), so the file holds 77 corrections.
- **Other importer fixes.** A `dc` with no Failure clause is a check, not a saving throw (Gelatinous Cube's Ooze Cube). Prose with a Unicode minus (baboon, jackal) now pairs with its structured dice.
- **ADR 0005** now describes what is built, including the inert Mimic gate, the "Otherwise" over-damage against targets at or under the HP gate, and the Hit-sentence tail gap.
- **Audit.** Three rounds with `deepseek-v4.1-flash` (haiku was unavailable). The first two raised problems that were fixed or documented. The third raised one stray clause on balor, fixed and checked directly. No round returned `APPROVE`; the user approved the commit. Accepted: resistance applies per entry rather than per damage type, which differs by at most 1 point on the Mimic.
- **Follow-up.** Effects stated in the Hit sentence after the damage (a condition, Hit Point maximum loss) are dropped silently. Ticket 040 covers it.
