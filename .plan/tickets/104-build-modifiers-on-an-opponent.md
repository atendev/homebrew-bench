---
id: 104
title: Build batch value of modifiers on an opponent
type: task
mode: AFK
status: open
blocked_by: [103]
claimed: false
---

## Question

Build the decision in [Batch value of debuffs](../closed/094-batch-value-of-debuffs.md): the policy scores a modifier whose bearer is an opponent, both as a rider under `on_hit` or `on_failure` and as a `target` modifier cast through `modifier_plan`, as ADR 0012's "Modifiers on an opponent" paragraph says.

## Notes

- Skills: `implement`, `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- Needs ticket 103 first: the save term uses `SAVE_POINT_VALUE`, and Bane's `subtract_dice` reads through 103's die mean.
- `bearer_value` (`tactics.rs` ~915): key each term on the bearer's side. On the actor's side the buff terms stay as they are. On an opponent: AC -n → damage `D × AC_POINT_VALUE × n`; a save penalty n (flat or die mean) → damage `S × SAVE_POINT_VALUE × n`; save Disadvantage → damage `S × SAVE_ADVANTAGE_VALUE`; attack penalty n → control `threat(bearer) × AC_POINT_VALUE × n`; attack Disadvantage → control `threat(bearer) × AC_POINT_VALUE × 5`. All × rounds. `BearerValue` gains `control`. Wrong-side pairings score 0. `f = 0` for an opponent bearer. `on_end` and lethargy count only on the actor's side. D and S come from `incoming_rolls(bearer, …)`.
- Riders (`effect_value` ~1267): `Effect::TimedModifier` with a `target` bearer under `on_hit` / `on_failure` scores through the same `bearer_value`, so the same-source check, horizon and keep apply, scaled by the caller's p_hit / p_fail, with the depth and `MAX_THREAT_NESTING` guards the `Condition` arm uses. No reason note.
- `modifier_plan` (~860, reached only by an ability whose effects are all modifiers): add living opponents as candidate bearers after the actor and allies, only when a harmful change scores above 0 on them, so Haste never picks an opponent. The score adds `w.control × value.control`. The reason reads as Haste's and lists the harmful terms.
- Tests: Slow's AC -2 and Dex -2 score as damage on a failed save; Bane's `subtract_dice 1d4` reads as -2.5 on attacks (control) and saves (damage); a homebrew `on_hit` "Disadvantage on attack rolls" rider scores `threat × 0.5 × rounds × p_hit` as control; a concentration rider is weighted by `k` with `f = 0` and `h` cut to the rounds left; a harmful modifier on an ally and a helpful one on an opponent score 0; the same-source guard holds on both the rider path and `modifier_plan` (a second Slow on a slowed foe scores 0 for those terms); a no-save `target`-only homebrew harmful modifier is cast through `modifier_plan` on an opponent, while Haste never picks one; Haste's plan and score are unchanged.
- Data: none. No fixture changes.
- Versions: bump the workspace one patch. `POLICY_VERSION` stays 4. First confirm no curve input carries a modifier that lands on an opponent and would score (Hypnotic Pattern's Speed 0 stays 0, since Speed isn't scored). Regenerate the curve; it must differ only in `engine_version`. If any cell moves, stop and report, since the decision then calls for v5.
- Docs: ADR 0012 was updated in ticket 094. Add a CHANGELOG line. `CONTEXT.md` is unchanged.

## Resolution

<Filled in at close.>
