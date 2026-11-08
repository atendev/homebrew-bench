---
id: 107
title: Build target teleport scoring
type: task
mode: AFK
status: open
blocked_by: [097]
claimed: false
---

## Question

Build the decision in [Batch value of a target teleport](../closed/097-batch-value-of-a-target-teleport.md): a `target` teleport scores `threat(target) × 0.25` as control under the conditions below, with the 0.25 shared with the push's short case.

## Notes

- Skills: `implement`, `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- `effect_value` (`bench-core/src/tactics.rs` ~1266) has `Effect::Teleport(_) => Value::default()`. Give the `target` subject the shape of the `Effect::ForcedMove` arm above it: a factor (0 when `depth > 3`), then `threat(target)` unless the factor is 0 or `threat_nesting` has reached `MAX_THREAT_NESTING`, and the product in `control`.
- The factor is 0.25, or 0 when the target holds no engagement, or when `needs_engaged` is false for the target's actions and bonus actions (the check `forced_move_factor` makes at ~1317). "Holds no engagement" means no combatant lists it and it lists nobody, as `band` reads both sides.
- Name the 0.25 once and use it from both `forced_move_factor` and the teleport. Don't change how a push scores.
- Under `on_hit` and `on_failure` the value is already scaled by the hit or failure chance.
- Tests: a melee-only engaged target scores `threat × 0.25` at a `Teleport.range` of 0, 30 and 120, with the same score at each; an unengaged target scores 0; a target with a ranged attack scores 0; a target teleport under `on_hit` is scaled by the hit chance; a `self` teleport scores as before.
- `POLICY_VERSION` stays 4, since nothing in the curve carries a target teleport. Check that claim again at build time. Bump the workspace version one patch, regenerate `data/baseline/curve.json`, and confirm it differs only in `engine_version`. If any cell moves, stop and reopen the version question.
- Add a `CHANGELOG.md` line.

## Resolution

<Filled in at close: the decision and the reason for it.>
