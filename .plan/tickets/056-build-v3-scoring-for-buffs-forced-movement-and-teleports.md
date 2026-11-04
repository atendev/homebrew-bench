---
id: 056
title: Build v3 scoring for buffs, forced movement and teleports
type: task
mode: AFK
status: open
blocked_by: [046, 058, 059]
claimed: false
---

## Question

Implement the scoring decided in [Batch policy scores buffs, forced movement and teleports](../closed/045-batch-policy-scores-buffs-forced-movement-and-teleports.md) in `bench-core/src/tactics.rs`: replace the zero-value arm for `TimedModifier`, `ForcedMove` and `Teleport` with the three formulas, add the `buff_horizon` constant, amend ADR 0012, and add tests per formula.

## Notes

The scoring ships in policy v3 together with the reactions change, so this waits for the reactions decision and its build. This ticket regenerates `data/baseline/curve.json`, once, after the reactions build, [Import legendary action cooldowns](../closed/058-import-legendary-action-cooldowns.md) and [Build upcast spell scaling](059-build-upcast-spell-scaling.md) have landed, so the policy, engine and fixtures all settle first. Check how far standard-party cells shift when caster Haste starts scoring, from a scratch run outside the repo. `TempHp` and `RollModifier` share the zero-value arm and stay at zero. Consult `tdd`, `codebase-design`, `domain-modeling`.

Ticket 057 left the curve stale on engine 0.1.1, ticket 058 changes the fixture set, and ticket 059 moves the engine to 0.1.2. Until this ticket regenerates the curve, two tests accept those stale reasons: `stale_beyond_the_engine` in `bench-core/tests/calibration.rs` and the CLI test `batch_compares_against_the_embedded_curve_outside_the_repo`. After the regeneration, remove both allowances so the tests again require a fresh curve.

## Resolution
