---
id: 056
title: Build v3 scoring for buffs, forced movement and teleports
type: task
mode: AFK
status: open
blocked_by: [046, 058, 059, 060]
claimed: false
---

## Question

Implement the scoring decided in [Batch policy scores buffs, forced movement and teleports](../closed/045-batch-policy-scores-buffs-forced-movement-and-teleports.md) in `bench-core/src/tactics.rs`: replace the zero-value arm for `TimedModifier`, `ForcedMove` and `Teleport` with the three formulas, add the `buff_horizon` constant, amend ADR 0012, and add tests per formula.

## Notes

The scoring ships in policy v3 together with the reactions change, so this waits for the reactions decision and its build. This ticket regenerates `data/baseline/curve.json`, once, after the reactions build, [Import legendary action cooldowns](../closed/058-import-legendary-action-cooldowns.md), [Build upcast spell scaling](059-build-upcast-spell-scaling.md) and [Build encounter lighting](060-build-encounter-lighting.md) have landed, so the policy, engine and fixtures all settle first. Check how far standard-party cells shift when caster Haste starts scoring, from a scratch run outside the repo. `TempHp` and `RollModifier` share the zero-value arm and stay at zero. Consult `tdd`, `codebase-design`, `domain-modeling`.

Ticket 057 left the curve stale on engine 0.1.1, ticket 058 changes the fixture set, and ticket 059 moves the engine to 0.1.2 and regenerates the standard party, which adds a "the party differs" reason. Ticket 060 changes the fixture set again (11 lighting traits) without moving batch numbers under the default `dim_light`, and the curve is generated in `dim_light`. Until this ticket regenerates the curve, two tests accept those three stale reasons: `stale_beyond_the_engine` in `bench-core/tests/calibration.rs` and the CLI test `batch_compares_against_the_embedded_curve_outside_the_repo`. After the regeneration, remove all three allowances (engine, fixture set and party) from both tests so they again require a fresh curve.

## Resolution
