---
id: 106
title: Build plan-failure warnings by role
type: task
mode: AFK
status: closed
blocked_by: [096]
claimed: false
---

## Question

Build the decision in [Plan-failure warnings split per level](../closed/096-plan-failure-warnings-split-per-level.md): role keys for party members, combatant ids in the error text replaced with labels, the level list on merged rows, and the CHANGELOG line.

## Notes

- Skills: `implement`, `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- `PlanFailure` (`bench-core/src/batch.rs` ~119) gains whether `combatant` is a role label, worked out in `Log::plan_failed` (~842), which already looks up the combatant and its stat block; `combatant` then holds the role label or the name. Rewrite `PlanFailure.error` there, resolving each combatant id through `state.combatants`, longest first, on whole tokens: the characters before and after the id are each the text's edge or not a letter, digit or `-`. The `plan failed` log lines format the engine error `e` directly (~715, ~720), not `PlanFailure.error`, so they stay unchanged. The level needs no new field: `plan_failure_warnings` reads `r.level` from the `FightResult` it iterates.
- Pull the role match out of `group_party` (~1083, `b.id.contains(a)` over `crate::party::ARCHETYPES`) into one helper both call. A role keys only when exactly one member of that level's party matches it.
- `plan_failure_warnings` (~488) keeps its tally key (`combatant`, `action`, `error`, `during`, `as_reaction`), now holding the label and the rewritten error, collects the levels per row, and prints `<Role> (level N | levels a-b, c)` for role-keyed rows only. Monster and name-keyed rows read as today, so the 070 tests keep passing unchanged. Sorting stays most frequent first.
- Tests: the reproduction (striker Longbow `70` at levels 3-5 gives one row naming `levels 3-5` with the summed count); an id-bearing error merging across levels; `-1` against `-10` token matching on both sides; a `--party` with two strikers keeping two rows; a roleless member keeping its name; the existing 070 tests still passing.
- Release builds of master and this branch must give byte-identical report numbers and logs for ogre at seed 7. Only warning rows may differ.

## Resolution

Built on 2026-11-10 in the `wayfinder-runner` skill's auto mode. A haiku auditor approved in one round, and the user chose commit and merge. No test CI exists: the only workflow is the `cargo-dist` release workflow.

`party::role_rank` is the one role match, shared by `group_party` and the new `failure_labels`. `Log::plan_failed` labels every combatant through `failure_labels`: a party member whose stat block id names a role that no other party-side combatant names is the capitalized role, everyone else is its name. `PlanFailure` gains `role_keyed`, and `replace_ids` rewrites combatant ids in the error text to those labels, longest id first on whole tokens. The `plan failed` log lines still format the raw engine error. `plan_failure_warnings` keys on `role_keyed` as well, collects each row's levels from `FightResult.level`, and prints `Striker (levels 3-5): ...` through `level_list` for role rows only.

Engine and policy versions unchanged. Release builds of master and this branch gave byte-identical output for ogre at seed 7. Seven tests were added, and the 474 core tests pass. Decision report: [106-implementation](../reports/106-implementation.md).

Left alone, from the audit: a monster named like a role label reads the same as a role row, and a party-side summon naming a role would block that role's key. Neither exists in batch.
