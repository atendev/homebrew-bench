---
id: 074
title: Regenerate the baseline for policy v3
type: task
mode: AFK
status: open
blocked_by: [056, 073, 076, 077, 078]
claimed: false
---

## Question

Split from [Build v3 scoring for buffs and forced movement](../closed/056-build-v3-scoring-for-buffs-forced-movement-and-teleports.md) for size. Regenerate `data/baseline/curve.json` exactly once, after 056, [073](../closed/073-build-v3-teleport-scoring-and-engine-sourced-dispositions.md), [076](076-build-haste-value-for-concentration-risk-and-short-fights.md), [077](077-build-ranged-attacks-in-close-combat.md) and [078](078-build-teleport-subject.md) land, and remove every stale-curve allowance. No engine or policy change here, so no version bump.

## Notes

Baseline: regenerate `data/baseline/curve.json` once, with lighting `unspecified`, after all of these have landed, so the policy, engine and fixtures settle first:
- [Import legendary action cooldowns](../closed/058-import-legendary-action-cooldowns.md) changed the fixture set.
- [Build upcast spell scaling](../closed/059-build-upcast-spell-scaling.md) moves the engine version and regenerates the standard party.
- [Build encounter lighting](../closed/060-build-encounter-lighting.md) changes the fixture set (11 lighting traits) and moves the engine version to 0.1.4, without moving numbers under `unspecified`.
- [Fix Advantage and Disadvantage cancellation on attack rolls](../closed/061-fix-advantage-disadvantage-cancellation.md) moves the engine version.
- [Build live reaction windows](../closed/062-build-live-reaction-windows.md) moves the engine version and changes the fixture set (Parry) and the party (Shield's trigger).
- [Build batch reactions policy](../closed/063-build-batch-reactions-policy.md) moves the policy to v3.
- [Build modifier bearers and Speed modifiers](../closed/065-build-modifier-bearers-and-speed-modifiers.md) moves the engine version and makes Speed modifiers count.
- [Build Haste extra action and lethargy](../closed/066-build-haste-extra-action-and-lethargy.md) moves the engine version.
- [Build attack-roll condition sources](../closed/068-build-attack-roll-condition-sources.md) moves the engine version.
- [Build saving-throw condition sources](../closed/069-build-saving-throw-condition-sources.md) moves the engine version.

After the regeneration, clean up the stale-curve allowances so the tests again require a fresh curve:
- Remove all four accepted reasons (engine, fixture set, party, policy) from `stale_beyond_the_engine` in `bench-core/tests/calibration.rs` and from `batch_compares_against_the_embedded_curve_outside_the_repo` in `bench/tests/cli.rs`.
- Remove the `#[ignore]` on `a_fixture_regenerates_to_its_committed_row`, whichever ticket added it (059, 061, 063, 065, 066, 068 or 069).

Report how the dragon cells moved between the old curve and the new one in the Resolution. [Do dragons still read above their CR on the v3 curve](064-do-dragons-still-read-above-cr-on-the-v3-curve.md) reads them next.
- Also landed since the list above was written: [Build massive damage at 0 Hit Points](../closed/071-build-massive-damage-at-zero-hit-points.md), [Batch warns on quiet plan failures](../closed/070-batch-warns-on-quiet-plan-failures.md) (warnings only), 056, 073 and [Build Haste value for concentration risk and short fights](076-build-haste-value-for-concentration-risk-and-short-fights.md) (076, which changes v3 buff scoring and the engine version). Remove any `#[ignore]` they added too.
- Still to land before the regeneration: [Build ranged attacks in close combat](077-build-ranged-attacks-in-close-combat.md) (077, an engine attack source and the engine version) and [Build teleport subject](078-build-teleport-subject.md) (078, the teleport vocabulary, policy teleport plans and the engine version). Remove any `#[ignore]` they add.

## Resolution
