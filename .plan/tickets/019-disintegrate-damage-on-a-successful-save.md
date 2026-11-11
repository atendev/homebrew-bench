---
id: 019
title: Disintegrate damage on a successful save
type: task
mode: AFK
status: open
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

<!-- Filled in at close. -->
