# Ticket 099 requirements report

Ticket: `.plan/tickets/099-build-bonus-action-disengage.md`. Depends on 090 (closed). Decision: `.plan/closed/090-bonus-action-disengage.md`.

## Requirements

1. `Effect::Disengage` is an additive variant at schema version 1. Applying it sets `action_economy.disengaged` through the same path as `bench disengage`. `opportunity_eligibility` (`engine.rs` ~1436) is unchanged.
2. A lone `disengage` effect needs no target. `bench action goblin-warrior nimble-escape` succeeds without `--target`, spends the bonus action, and leaves the action unspent.
3. The event text names the ability: "goblin-warrior uses Nimble Escape to Disengage". It does not say "takes the Disengage action" (`engine.rs` ~387, ~396).
4. The importer maps the four "takes the … Disengage … action" wordings to `disengage` on bonus-action entries only. That covers Hasten's plural "takes the Dash and Disengage actions". It yields 11 fixtures: goblin warrior, goblin minion, goblin boss, panther, tiger, saber-toothed tiger (Nimble Escape); assassin, spy (Cunning Action); vampire spawn, vampire familiar (Deathless Agility); clay golem (Hasten, keeps Recharge 5–6).
5. Each of those 11 entries is marked partly modeled. Hide and Dash stay as agent clauses in the same sentence. Prowl (weretiger) stays unmodeled.
6. The import review (`data/srd/import-review.md` and `.json`) records the partly modeled status and the agent clauses.
7. ADR 0008 lists prose read as data. It gains the Disengage exception beside the Flyby/Agile exception.
8. Policy: a standalone Disengage is never chosen. `choose(Bonus)` does not fall into the generic target loop. Disengage is emitted only with an action plan that steps out, and it scores 0 otherwise.
9. In `plan_for`'s step-out branch (`tactics.rs` ~693), a ready Disengage sets the cost to the lower of `w.survival × oa` and the score of the best plan it gives up. The given-up plan is the best other bonus plan on the pre-action state, or the best extra-action plan.
10. When a bonus Disengage and an extra action that allows Disengage are both ready at equal cost, the extra action is chosen.
11. Batch commits a chosen Disengage before the action's move. The extra-action choice is made before the Action, Bonus, Extra order (`batch.rs` ~691) runs.
12. `teleport_value`'s avoided-attack count is capped when a free Disengage was available.
13. Hasten's Recharge 5–6 counts as a resource cost in the plan. `pick_best`'s limited-against-at-will rule does not block it.
14. The reason line names the ability that avoided the attacks, for example "steps out of melee, nimble-escape avoids 4.2 opportunity damage".
15. Retry path: when the action plan fails after Disengage is spent, the recompute sees the flag set and charges a penalty of 0.
16. `POLICY_VERSION` is 4. The assert near `tactics.rs` ~2435 is updated to match.
17. Batch findings (`batch.rs` ~352) no longer label `no_provoke` or `disengage` abilities "Not modeled in phase 1".
18. ADR 0005 gains `disengage` in its effect vocabulary.
19. ADR 0012: line 11 ("The policy never plans the Disengage action") and line 13 ("and never Disengage") both read "only to step out without provoking".
20. `live-mode.md` (~68) gains three lines: a bonus Disengage avoids Opportunity Attacks without spending the action; when the DM picks Hide or Dash, an override sets `bonus_action_used`; Hasten's Dash half is the DM's call.
21. `CHANGELOG.md` gains a line for this change.
22. Replay: a live encounter with a goblin, saved on 0.1.24, refolds to its recorded state hashes. Its initial state holds the old unmodeled Nimble Escape, and the refold must not change that.
23. The engine patch version is bumped. `bench baseline generate` regenerates the fixtures and `data/baseline/curve.json`. The calibration row is checked.
24. Among the 11 fixtures, only the assassin's cells change. If a goblin, spy or melee-only fixture changes, the cause is found before commit.

## Seams under test

- Engine step and `bench action` CLI: asserts items 1 to 3. The flag matches the one `bench disengage` sets, the target-less command resolves, bonus action is spent and action is not, and the event text names the ability.
- Importer output for the 11 fixtures: asserts items 4 and 5. Each entry carries `disengage` on its bonus-action entry, is partly modeled, keeps Hide and Dash as agent clauses, and Prowl stays unmodeled.
- Import review: asserts item 6 for the same 11 entries.
- `choose(Bonus)` and `plan_for` output: asserts items 8 to 10 and 12 to 13. A standalone Disengage is never chosen, cost is the lower of the two figures, ties go to the extra action, the teleport value is capped, and Hasten's cost includes its recharge.
- Batch ordering (`batch.rs` ~691): asserts item 11. The Disengage is committed before the action's move.
- Reason line: asserts item 14. The string contains the avoiding ability's name and the avoided damage figure.
- Retry path: asserts item 15. After a failed action plan with Disengage spent, the recompute gives a penalty of 0.
- Batch findings text: asserts item 17 for `no_provoke` and `disengage`.
- Replay of a saved 0.1.24 goblin encounter: asserts item 22. The recorded state hashes match after refold. A v3 `--seed --fight` rerun is not expected to match on a v4 binary.
- Baseline generation: asserts items 23 and 24. The calibration row is present, and the per-cell diff across the 11 fixtures moves only for the assassin.
- Docs (items 18 to 21): checked by review and text search, not by a behavioural test.

## Out of scope

- Dash as a movement budget.
- Hide as a standalone bonus action.
- Prowl (weretiger).
- Hit-and-run.

Note: the 090 resolution lists a CONTEXT.md glossary update (Bonus action; Disengage covering action, bonus and extra action forms). The 099 Notes do not mention it. It is not a requirement here. Confirm before close.
