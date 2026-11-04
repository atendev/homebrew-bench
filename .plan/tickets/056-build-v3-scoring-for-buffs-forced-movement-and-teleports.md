---
id: 056
title: Build v3 scoring for buffs, forced movement and teleports
type: task
mode: AFK
status: open
blocked_by: [046, 058, 059, 060, 061, 062, 063, 065, 066, 068, 069]
claimed: false
---

## Question

Implement the scoring decided in [Batch policy scores buffs, forced movement and teleports](../closed/045-batch-policy-scores-buffs-forced-movement-and-teleports.md) in `bench-core/src/tactics.rs`: replace the zero-value arm for `TimedModifier`, `ForcedMove` and `Teleport` with the three formulas, add the `buff_horizon` constant, amend ADR 0012, and add tests per formula. Then regenerate the baseline curve once for policy v3.

## Notes

Consult `tdd`, `codebase-design`, `domain-modeling`.

Policy v3 has two halves. [Build batch reactions policy](../closed/063-build-batch-reactions-policy.md) lands first: it moves `POLICY_VERSION` to 3, and batch starts taking Opportunity Attacks, Shield and Parry ([Batch policy takes reactions](../closed/046-batch-policy-takes-reactions.md)). This ticket adds the scoring half under the same v3. Don't bump `POLICY_VERSION` again; neither half ships a release alone.

Scoring:
- Buffs use 063's named constant for one AC point (`tactics::AC_POINT_VALUE`, 0.1) instead of a second literal.
- Buffs land on a chosen bearer, not only the actor ([Haste extra action and ally-targeted buffs](../closed/055-haste-extra-action-and-ally-targeted-buffs.md), decision 6, built on ticket 065's `bearer`). For a top-level `target` modifier, value 045's formula for the actor and each living ally, reading `incoming` and `threat` on that bearer, and plan the cast on the best one. Ties go to the actor, then to the lower id. Every ally counts as in range, as for heals. A bearer that already has an active modifier with the same `source_ability` scores 0 for it. A `self` modifier is valued on the actor only. A modifier whose bearer would be an opponent (under a save's `on_failure` or an attack's `on_hit`) scores 0: debuff values are not decided. Log the bearer in the plan reason, for example `standard-caster-05: haste on standard-striker-05 (+2 AC, incoming 14.0)`. Tests: the level 5 caster hastes the ally with the highest incoming damage, not itself; a second Haste plan on an already hasted ally scores 0; Slow's riders add no buff value to the caster.
- Haste's extra action ([Build Haste extra action and lethargy](066-build-haste-extra-action-and-lethargy.md)). Value: an `extra_action` modifier adds, to the damage half of the buff score, the bearer's best expected damage from one `one_attack`-eligible action against its top-priority opponent, times `min(rounds, buff_horizon)`. Lethargy is not scored: it costs only when concentration breaks inside the horizon, and the existing `breaks_own_concentration` check already covers the caster. Use: the batch runner asks the policy for an extra-action plan after the action and bonus sections when the combatant has an extra action, through the engine's own check. The plan is the best eligible single attack, with movement only if the action plan left movement; no Disengage plans (as decided in 046). Tests: the level 5 caster hastes the striker over itself once the extra attack counts; a Hasted striker makes one extra attack per turn in batch and never a Multiattack; with Haste's concentration broken, the striker loses its next turn.
- Teleports score the Opportunity Attacks they avoid by calling 063's penalty function (`Ctx::step_out_penalty`): eligible engaged reactors the policy would attack with, each at its `opportunity_value`. This replaces ticket 045's "sum of `best_single_attack` over engaged foes that still have their reaction", because 046 decided the mover prices only the attacks the policy really takes. The penalty and the reserve return 0 at or past `MAX_THREAT_NESTING` (063 builds that guard), because the teleport score calls the penalty, which reaches `threat()` and `ability_value` and can come back to a teleport. Keep the guard, and add a test: a stat block whose actions include a teleport and a Multiattack that uses it, engaged with a reactor that has Parry, gets a plan without overflowing the stack, as the Dryad test does for control.
- `TempHp` and `RollModifier` share the zero-value arm and stay at zero.
- Fix `disposition` in `tactics.rs`: it sums and clamps, so an attacker with two Advantage sources and one Disadvantage source scores as Advantage. Use the two-flag rule from ticket 061 (both or neither is straight), with a test that two of one and one of the other score straight.
- The policy's `disposition` must also read the sources [Align attack-roll condition sources](../closed/067-align-attack-roll-condition-sources-with-2024-rules.md) added (ADR 0017): Prone by engagement, Grappled and Frightened by `source`, Invisible through the sight rule, downed PCs as Unconscious. Call the engine's `disposition` (ticket [068](068-build-attack-roll-condition-sources.md) makes it `pub(crate)`) instead of keeping a second list, reading engagement as the plan leaves it when the plan moves, and test that a ranged plan against a Prone target scores Disadvantage.
- Check how far standard-party cells shift when caster Haste starts scoring, from a scratch run outside the repo, and record it in the Resolution.
- The batch report's method sentence: rewrite the "gives buffs and other unscored effects no value" half. 063 already rewrote the reactions half.

Baseline: regenerate `data/baseline/curve.json` once, with lighting `unspecified`, after all of these have landed, so the policy, engine and fixtures settle first:
- [Import legendary action cooldowns](../closed/058-import-legendary-action-cooldowns.md) changed the fixture set.
- [Build upcast spell scaling](059-build-upcast-spell-scaling.md) moves the engine version and regenerates the standard party.
- [Build encounter lighting](../closed/060-build-encounter-lighting.md) changes the fixture set (11 lighting traits) and moves the engine version to 0.1.4, without moving numbers under `unspecified`.
- [Fix Advantage and Disadvantage cancellation on attack rolls](061-fix-advantage-disadvantage-cancellation.md) moves the engine version.
- [Build live reaction windows](../closed/062-build-live-reaction-windows.md) moves the engine version and changes the fixture set (Parry) and the party (Shield's trigger).
- [Build batch reactions policy](../closed/063-build-batch-reactions-policy.md) moves the policy to v3.
- [Build modifier bearers and Speed modifiers](065-build-modifier-bearers-and-speed-modifiers.md) moves the engine version and makes Speed modifiers count.
- [Build Haste extra action and lethargy](066-build-haste-extra-action-and-lethargy.md) moves the engine version.
- [Build attack-roll condition sources](068-build-attack-roll-condition-sources.md) moves the engine version.
- [Build saving-throw condition sources](069-build-saving-throw-condition-sources.md) moves the engine version.

After the regeneration, clean up the stale-curve allowances so the tests again require a fresh curve:
- Remove all four accepted reasons (engine, fixture set, party, policy) from `stale_beyond_the_engine` in `bench-core/tests/calibration.rs` and from `batch_compares_against_the_embedded_curve_outside_the_repo` in `bench/tests/cli.rs`.
- Remove the `#[ignore]` on `a_fixture_regenerates_to_its_committed_row`, whichever ticket added it (059, 061, 063, 065, 066, 068 or 069).

Report how the dragon cells moved between the old curve and the new one in the Resolution. [Do dragons still read above their CR on the v3 curve](064-do-dragons-still-read-above-cr-on-the-v3-curve.md) reads them next.

## Resolution
