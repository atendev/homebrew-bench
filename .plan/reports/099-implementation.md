# Ticket 099 implementation report

Ticket: `.plan/tickets/099-build-bonus-action-disengage.md`. Requirements: `.plan/reports/099-requirements.md`. Diff reviewed: `target/099-audit-2.diff` (the data changes, the 11 fixtures and the import review, are summarized here, not in the diff). Branch `ticket-099-build-bonus-action-disengage`, uncommitted changes.

## What was built

- `bench-core/src/effect.rs`: additive `Effect::Disengage` variant at schema version 1.
- `bench-core/src/engine.rs`: an ability with a `disengage` effect calls `set_disengaged`, the same flag path as `bench disengage`, and logs `<actor> uses <Ability> to Disengage`. A lone `disengage` effect needs no target. `opportunity_eligibility` is unchanged.
- `bench-core/src/tactics.rs` (policy v4):
  - `Plan.disengage: Option<DisengageBy>` (`Bonus(id)` or `Extra`), skipped in JSON when absent.
  - `disengage_option` finds the cheapest ready Disengage: a limited bonus Disengage costs its resource weight, and the extra action wins ties. Its cost is the best other plan, scored on the pre-action state.
  - `plan_for`'s step-out branch pays the lower of `survival × oa` and that cost. `only_disengages` stops a Disengage-only ability from being planned alone.
  - `teleport_value` is capped at the same Disengage cost.
  - The reason line names the avoiding ability.
  - `POLICY_VERSION` is 4, and the fingerprint assert is updated.
- `bench-core/src/batch.rs`:
  - `execute` runs the chosen Disengage before the move, and a failed plan returns the Disengage events it already spent. The fight log shows them.
  - The findings no longer call `no_provoke` or `disengage` abilities "Not modeled in phase 1". The skipped-mechanic warning drops the `no_provoke` case.
  - The HTML method text names the Disengage limit.
- `bench-core/src/live.rs`: no code change. The new replay test is added.
- `scripts/import-srd-monsters.py`: `disengage_sentence` maps the four "takes the … Disengage … action" wordings on bonus actions only. Each is partly modeled, with Hide and Dash as an agent clause and a class (d) review note. Prowl stays unmodeled.
- Fixtures: 11 regenerated (goblin warrior, minion and boss, panther, tiger, saber-toothed tiger, assassin, spy, vampire spawn, vampire familiar, clay golem). `data/srd/import-review.{md,json}` records them.
- Engine version 0.1.25. `data/baseline/curve.json` regenerated with `bench baseline generate`.
- Docs: ADR 0005 gains `disengage`. ADR 0008 gains the exception beside Flyby and Agile. ADR 0012 lines 11 and 13 now read "only to step out without provoking". `live-mode.md` gains the three lines. `batch-mode.md` gets one sentence on the policy. `CHANGELOG.md` has the Added and Changed entries.
- `bench-core/tests/data/engine-0.1.24-state.json`: a real 0.1.24 encounter with a goblin, recorded with the 0.1.24 binary.

## Requirements coverage

- 1 to 3, flag, target-less command, event text: met. The engine test and the CLI test check the flag, the bonus action spent, the action unspent, and the log line.
- 4 to 6, importer and review: met. The 11 fixtures carry `disengage` on their bonus-action entry, partly modeled. The review records the class (d) note. No automated test covers the importer output in this diff.
- 7, ADR 0008: met.
- 8, never standalone: met. `only_disengages` and the `choose(Bonus)` check.
- 9, cost is the lower of the two: met.
- 10, ties go to the extra action: met.
- 11, batch commits before the move: met by `execute`. The batch order (`batch.rs` ~691) is unchanged, since the extra-action choice is made in the plan.
- 12, teleport cap: met.
- 13, Hasten's recharge counts: met. The at-will rule does not block a recharge Disengage.
- 14, reason line: met.
- 15, retry path: met. A failed plan recomputes with the flag set and penalty 0.
- 16, `POLICY_VERSION` 4: met.
- 17, findings text: met.
- 18 to 21, ADRs, live-mode reference, CHANGELOG: met.
- 22, replay: met. The 0.1.24 goblin encounter refolds to its recorded hashes.
- 23, engine version and regenerated curve: met. The calibration row passes.
- 24, only the assassin's cells move: not met as written. The vampire familiar moves as well (see below).

## Tests added

- `bench-core/src/engine.rs`: `a_bonus_disengage_needs_no_target_and_leaves_the_action_unspent`. Asserts the event text, the flag, the bonus action spent, the action unspent, a move refused with the disengaged reason, and a second use refused.
- `bench/tests/cli.rs`: `nimble_escape_disengages_as_a_bonus_action_with_no_target`. Runs `bench action goblin-warrior nimble-escape` with no target and asserts the line, the economy, and that replay matches.
- `bench-core/src/live.rs`: `an_engine_0_1_24_goblin_refolds_with_its_unmodeled_nimble_escape`. Loads the 0.1.24 state, checks the old Nimble Escape is unmodeled, and asserts replay matches.
- `bench-core/src/tactics.rs`:
  - `a_free_bonus_disengage_steps_out_without_the_penalty`: no standalone choice, the plan is Disengage-first, and the reason names `nimble-escape avoids`.
  - `a_disengage_that_gives_up_a_better_bonus_plan_is_not_taken`.
  - `an_extra_action_disengage_goes_first_at_equal_cost`.
  - `a_recharge_disengage_counts_its_resource_cost`: Hasten's cost moves the decision, and an at-will Nimble Escape listed after it costs nothing.
  - `after_a_spent_disengage_the_recompute_steps_out_for_free`.
  - `a_free_disengage_caps_what_a_teleport_avoids`.
  - The fingerprint test now asserts version 4.
- `bench-core/src/batch.rs`:
  - `no_provoke_and_disengage_abilities_are_not_called_unmodeled`.
  - `the_assassin_disengages_with_cunning_action_before_it_steps_out`: Disengage is the first event, no Opportunity Attack, bonus action spent.
  - `a_disengage_spent_before_a_failed_plan_leaves_a_free_step_out`: the failed plan still reports the Disengage it spent, and the retry is free.

## Deviations and choices

- Only action plans Disengage first. Bonus and legendary plans never do, and the teleport cap also reads only action plans.
- The teleport cap applies to action plans. A teleport in another section keeps its full value.
- The curve moves more than requirement 24 expected. The assassin moves in 19 cells, and the vampire familiar in 10. The familiar's Umbral Dagger is melee-or-thrown, so it now Disengages from the creature it is engaged with to throw at a PC it isn't engaged with. Before, it stayed in melee and took the close-combat Disadvantage. That is the intended effect, and the changelog records it. The other nine fixtures changed only in `ignored_abilities`. Pooled results moved in 24 cells.
- A goblin batch now needs its Nimble Escape agent clause resolved, like the 207 other partly modeled fixture abilities.
- Move events of a failed plan stay unlogged, as before. Only the Disengage that was already spent is logged, which fixes the lost line from audit round 1.

## Audit

- Round 1 (haiku) raised 7 points. Fixed: the lost Disengage log line on a failed plan, a misplaced doc comment, the Haste reason naming, and choosing the cheapest bonus Disengage. The familiar curve change went into the CHANGELOG. The CONTEXT.md glossary point was rejected, since commit 0c01997 already made that change.
- Round 2 left the ADR 0008 count of exceptions.
- Round 3 dropped the count and approved.

## Verification

- `cargo test --workspace` passes: 465 tests, calibration included.

## Left for the user

- Confirm the familiar's curve move is wanted. It is the one place the data contradicts requirement 24.
- Add an automated test for the importer output and the import review. The 11 fixtures are checked only by regeneration.
- The importer header in `scripts/import-srd-monsters.py` still counts "five exceptions", while ADR 0008 no longer gives a count. Keep one or the other consistent.
- Commit the change. The ticket's Resolution section is still empty.

## Follow-ups

- Hasten's Dash half stays an agent clause, as the requirements set.
