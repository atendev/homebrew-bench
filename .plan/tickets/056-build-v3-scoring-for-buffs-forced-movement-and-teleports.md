---
id: 056
title: Build v3 scoring for buffs, forced movement and teleports
type: task
mode: AFK
status: open
blocked_by: [046, 058, 059, 060, 061, 062]
claimed: false
---

## Question

Implement the scoring decided in [Batch policy scores buffs, forced movement and teleports](../closed/045-batch-policy-scores-buffs-forced-movement-and-teleports.md) in `bench-core/src/tactics.rs`: replace the zero-value arm for `TimedModifier`, `ForcedMove` and `Teleport` with the three formulas, add the `buff_horizon` constant, amend ADR 0012, and add tests per formula.

## Notes

The scoring ships in policy v3 together with the reactions change, so this waits for the reactions decision and its build. This ticket regenerates `data/baseline/curve.json`, once, after the reactions build, [Import legendary action cooldowns](../closed/058-import-legendary-action-cooldowns.md), [Build upcast spell scaling](059-build-upcast-spell-scaling.md) and [Build encounter lighting](060-build-encounter-lighting.md) and [Fix Advantage and Disadvantage cancellation on attack rolls](061-fix-advantage-disadvantage-cancellation.md) have landed, so the policy, engine and fixtures all settle first. Check how far standard-party cells shift when caster Haste starts scoring, from a scratch run outside the repo. `TempHp` and `RollModifier` share the zero-value arm and stay at zero. Policy v3 also fixes `disposition` in `tactics.rs`: it sums and clamps, so an attacker with two Advantage sources and one Disadvantage source scores as Advantage. Use the two-flag rule from ticket 061 (both or neither is straight), with a test that two of one and one of the other score straight. Consult `tdd`, `codebase-design`, `domain-modeling`.

Ticket 057 left the curve stale on engine 0.1.1, ticket 058 changes the fixture set, tickets 059 and 061 each move the engine version, and 059 regenerates the standard party, which adds a "the party differs" reason. Ticket 060 changes the fixture set again (11 lighting traits) without moving batch numbers under the default `unspecified` lighting, and the curve is generated with lighting `unspecified`. Ticket [062](062-build-live-reaction-windows.md) moves the engine version, changes the fixture set (Parry) and the party (Shield's trigger) without moving batch numbers. Until this ticket regenerates the curve, two tests accept those three stale reasons: `stale_beyond_the_engine` in `bench-core/tests/calibration.rs` and the CLI test `batch_compares_against_the_embedded_curve_outside_the_repo`. After the regeneration, remove all three allowances (engine, fixture set and party) from both tests so they again require a fresh curve.

## Resolution
