---
id: 019
title: Disintegrate damage on a successful save
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Set Disintegrate's save in `data/spells/effects.json` to deal no damage on a success, and regenerate the monster curve if its cells move.

## Notes

- Skills: `tdd`, `code-review`.
- Found while resolving [Model more SRD cantrips](../closed/014-model-more-srd-cantrips.md). A save with `half_on_success` and `no_damage_on_success` both false deals full damage on a success (`engine.rs`, the save branch of effect resolution). Disintegrate is the only entry with damage written that way, so a successful save takes 10d6 + 40.
- The SRD text gives damage only "on a failed save". `scripts/scaffold-spells.py` now defaults a damaging save without half-damage wording to `no_damage_on_success: true`, so a fresh scaffold of Disintegrate is right.
- The standard caster knows Disintegrate from party level 11 (`data/party/standard-spells.json`), so the fix can move curve cells at levels 11 to 20. Run the calibration check and regenerate with `--threads 4` or lower only if cells move.
- It blocks [Spell baseline](016-spell-baseline.md): level-6 lift would otherwise count full damage on a success.

## Resolution

Disintegrate's save sets `no_damage_on_success: true` (engine 0.1.45), so a successful Dexterity save takes no damage, as the SRD gives damage only on a failure. The engine's default for a save with neither flag stays, since Disintegrate was the only entry written that way.

- A new test walks the effects file and fails on any damaging save that sets neither `half_on_success` nor `no_damage_on_success`, nested saves included.
- Cells moved, so the curve and the spell panel were regenerated with `--threads 4`. The curve moves at levels 11 to 20 only: 202 fixture cells change their win count, 193 of them downward, by up to 43 of 100. The biggest falls are against adult and ancient dragons, and the cause is not investigated. Legendary Resistance is not it, since the fixtures hold it unmodeled.
- The panel's members change at party levels 13, 15 and 17, where the control party keeps Disintegrate. Its spellcasting fixtures there become the Adult Red Dragon, the Adult Gold Dragon and the Ancient Copper Dragon.

Decision report: [019-decisions](../reports/019-decisions.md).
