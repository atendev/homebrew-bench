---
id: 056
title: Build v3 scoring for buffs and forced movement
type: task
mode: AFK
status: open
blocked_by: [046, 058, 059, 060, 061, 062, 063, 065, 066, 068, 069, 071]
claimed: false
---

## Question

Implement the buff and forced-movement scoring decided in [Batch policy scores buffs, forced movement and teleports](../closed/045-batch-policy-scores-buffs-forced-movement-and-teleports.md) in `bench-core/src/tactics.rs`: replace the zero-value arm for `TimedModifier` and `ForcedMove` with 045's formulas, add the `buff_horizon` constant, value Haste's extra action and let batch use it, amend ADR 0012, and add tests per formula.

This ticket was split for size on 2026-11-04. Teleport scoring and the policy reading the engine's attack and save sources moved to [Build v3 teleport scoring and engine-sourced policy dispositions](073-build-v3-teleport-scoring-and-engine-sourced-dispositions.md). The one baseline regeneration moved to [Regenerate the baseline for policy v3](074-regenerate-the-baseline-for-policy-v3.md). Don't regenerate `data/baseline/curve.json` here. If `a_fixture_regenerates_to_its_committed_row` differs, add `#[ignore = "curve stale until ticket 074 regenerates it"]`. `POLICY_VERSION` stays 3. Batch numbers change, so bump the workspace version one patch.

## Notes

Consult `tdd`, `codebase-design`, `domain-modeling`.

Policy v3 has two halves. [Build batch reactions policy](../closed/063-build-batch-reactions-policy.md) lands first: it moves `POLICY_VERSION` to 3, and batch starts taking Opportunity Attacks, Shield and Parry ([Batch policy takes reactions](../closed/046-batch-policy-takes-reactions.md)). This ticket adds the scoring half under the same v3. Don't bump `POLICY_VERSION` again; neither half ships a release alone.

Scoring:
- Buffs use 063's named constant for one AC point (`tactics::AC_POINT_VALUE`, 0.1) instead of a second literal.
- Buffs land on a chosen bearer, not only the actor ([Haste extra action and ally-targeted buffs](../closed/055-haste-extra-action-and-ally-targeted-buffs.md), decision 6, built on ticket 065's `bearer`). For a top-level `target` modifier, value 045's formula for the actor and each living ally, reading `incoming` and `threat` on that bearer, and plan the cast on the best one. Ties go to the actor, then to the lower id. Every ally counts as in range, as for heals. A bearer that already has an active modifier with the same `source_ability` scores 0 for it. A `self` modifier is valued on the actor only. A modifier whose bearer would be an opponent (under a save's `on_failure` or an attack's `on_hit`) scores 0: debuff values are not decided. Log the bearer in the plan reason, for example `standard-caster-05: haste on standard-striker-05 (+2 AC, incoming 14.0)`. Tests: the level 5 caster hastes the ally with the highest incoming damage, not itself; a second Haste plan on an already hasted ally scores 0; Slow's riders add no buff value to the caster.
- Haste's extra action ([Build Haste extra action and lethargy](../closed/066-build-haste-extra-action-and-lethargy.md)). Value: an `extra_action` modifier adds, to the damage half of the buff score, the bearer's best expected damage from one `one_attack`-eligible action against its top-priority opponent, times `min(rounds, buff_horizon)`. Lethargy is not scored: it costs only when concentration breaks inside the horizon, and the existing `breaks_own_concentration` check already covers the caster. Use: the batch runner asks the policy for an extra-action plan after the action and bonus sections when the combatant has an extra action, through the engine's own check. The plan is the best eligible single attack, with movement only if the action plan left movement; no Disengage plans (as decided in 046). Tests: the level 5 caster hastes the striker over itself once the extra attack counts; a Hasted striker makes one extra attack per turn in batch and never a Multiattack; with Haste's concentration broken, the striker loses its next turn.
- The extra-action section this ticket adds to the batch runner must route every `execute` failure through `Log::plan_failed` with its own noun (`"extra action"`), and mark the failure lost when no plan ran in that slot, as `take_turn` and `legendary_window` do, so its faults reach the summary warnings ([Batch warns when plans fail or casts are never ready](../closed/070-batch-warns-on-quiet-plan-failures.md)).
- `TempHp` and `RollModifier` share the zero-value arm and stay at zero.
- Check how far standard-party cells shift when caster Haste starts scoring, from a scratch run outside the repo, and record it in the Resolution.
- The batch report's method sentence: rewrite the "gives buffs and other unscored effects no value" half. 063 already rewrote the reactions half.

## Resolution
