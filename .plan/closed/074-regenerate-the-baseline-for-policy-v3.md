---
id: 074
title: Regenerate the baseline for policy v3
type: task
mode: AFK
status: closed
blocked_by: [056, 073, 076, 077, 078]
claimed: false
---

## Question

Split from [Build v3 scoring for buffs and forced movement](../closed/056-build-v3-scoring-for-buffs-forced-movement-and-teleports.md) for size. Regenerate `data/baseline/curve.json` exactly once, after 056, [073](../closed/073-build-v3-teleport-scoring-and-engine-sourced-dispositions.md), [076](../closed/076-build-haste-value-for-concentration-risk-and-short-fights.md), [077](../closed/077-build-ranged-attacks-in-close-combat.md) and [078](../closed/078-build-teleport-subject.md) land, and remove every stale-curve allowance. No engine or policy change here, so no version bump.

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
- Also landed since the list above was written: [Build massive damage at 0 Hit Points](../closed/071-build-massive-damage-at-zero-hit-points.md), [Batch warns on quiet plan failures](../closed/070-batch-warns-on-quiet-plan-failures.md) (warnings only), 056, 073 and [Build Haste value for concentration risk and short fights](../closed/076-build-haste-value-for-concentration-risk-and-short-fights.md) (076, which changes v3 buff scoring and the engine version; it added no `#[ignore]`, as the calibration row still matched). Remove any `#[ignore]` they added too.
- Still to land before the regeneration: [Build ranged attacks in close combat](../closed/077-build-ranged-attacks-in-close-combat.md) (077, an engine attack source and the engine version) and [Build teleport subject](../closed/078-build-teleport-subject.md) (078, the teleport vocabulary, policy teleport plans and the engine version). Remove any `#[ignore]` they add.
- The committed goblin-warrior row is degenerate. Both its attacks carry agent clauses, and baseline generation ignores unresolved attacks, so the row shows 0 DPR, 1 round and 100 wins at every level, and `a_fixture_regenerates_to_its_committed_row` passes trivially. Switch that test to a fixture whose attacks are fully modeled. Found by the 077 audit.

## Resolution

Regenerated `data/baseline/curve.json` once with `bench baseline generate` (engine 0.1.15, policy v3, lighting `unspecified`, seed 20261102, 341 fixtures, 620 cells, 60 thin). No engine or policy change, so no version bump. `bench baseline status` now reports no reasons.

Cleanup:
- Removed `stale_beyond_the_engine` from `bench-core/tests/calibration.rs`; the curve and comparison tests now require an empty stale list.
- The embedded-curve test in `bench/tests/cli.rs` (`batch_compares_against_the_embedded_curve_outside_the_repo`) now requires `baseline status` to give no reasons.
- No `#[ignore]` was left on `a_fixture_regenerates_to_its_committed_row`; none of 056, 073, 076, 077 or 078 added one. The only `#[ignore]` in the repo is `from_source_with_real_cargo` in `bench/tests/install.rs`, which is unrelated.
- The regeneration test now uses `ogre` (dpr 4.9 at level 1, 85 wins) instead of the degenerate goblin warrior.

Dragon cells, measured as the lowest party level with 90 or more wins out of 100 (old curve to new curve). 21 means no level from 1 to 20 reached 90 wins.
- The 43 dragon-named fixtures (this includes the half-dragon, dragon turtle and pseudodragon): 23 read easier, 0 read harder, the median shift is one level easier.
- Adult dragons move 0 to 4 levels easier: black 13 to 11, brass 13 to 11, bronze 15 to 12, copper 14 to 11, gold 20 to 16, green 16 to 13, red 17 to 15, silver 17 to 13, white 15 to 12. Blue stays at 11.
- Ancient dragons: black 21 to 17, brass 21 to 17, copper 21 to 17, white 21 to 18, blue 19 to 18. Bronze, gold, green, red and silver stay at 21.
- Young dragons move 0 to 1 levels easier (blue 8 to 7, white 9 to 8, gold and red 12 to 11), wyrmlings 0 to 1.
- Pit fiend (CR 20) goes 21 to 19 and lich (CR 21) goes 21 to 18.
- Across all 341 fixtures, 66 read easier and 2 read harder (ettin and lamia, 3 to 4), with a median shift of 0. The easier shift is broader than dragons: low-CR fixtures such as the grick, priest and allosaurus also move from 2 to 1, so this ticket does not say why.
- The ogre row shows a median of 0.0 DPR at levels 5 and 7 to 20 with 1 to 2 median rounds, as the knight row does from level 10. It is probably the party ending the fight before the monster acts, but the curve alone doesn't confirm it.

[Do dragons still read above their CR on the v3 curve](064-do-dragons-still-read-above-cr-on-the-v3-curve.md) reads them next.
