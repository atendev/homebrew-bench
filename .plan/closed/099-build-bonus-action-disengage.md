---
id: 099
title: Build bonus-action Disengage
type: task
mode: AFK
status: closed
blocked_by: [090]
claimed: false
---

## Question

Build the decision in [Bonus-action Disengage](../closed/090-bonus-action-disengage.md): the `disengage` effect and its import, the live-mode command and log line, policy v4 Disengage before a step-out, the findings fix, the ADR amendments, and the regenerated curve.

## Notes

- Skills: `implement`, `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- Effect: `Effect::Disengage`, an additive variant at schema version 1. It sets `action_economy.disengaged` through the same path as `bench disengage`, so `opportunity_eligibility` (engine.rs ~1436) needs no change. A lone `disengage` effect needs no target, so `bench action goblin-warrior nimble-escape` must resolve without `--target`. Test it.
- Event text names the ability ("goblin-warrior uses Nimble Escape to Disengage"), not "takes the Disengage action" (engine.rs ~387, ~396).
- Importer (`scripts/import-srd-monsters.py`): match the four "takes the … Disengage … action" wordings on bonus actions only, including Hasten's plural "takes the Dash and Disengage actions" (11 fixtures in all), mark them partly modeled, and keep the rest of the sentence (Hide, Dash) as an agent clause. Record this in the import review. ADR 0008 lists prose read as data, so add it beside the Flyby/Agile exception. Prowl stays unmodeled.
- Policy (`tactics.rs`), with `POLICY_VERSION` going to 4 (update the assert near tactics.rs:2435):
  - Give Disengage its own plan path. It is emitted only together with an action plan that steps out, and it scores 0 otherwise, so `choose(Bonus)` must not fall into the generic target loop.
  - In `plan_for`'s step-out branch (~693), a ready Disengage changes the cost from `w.survival × oa` to the lower of that and the score of the best plan the Disengage gives up. That is the best other bonus plan, estimated on the pre-action state, or the best extra-action plan. When both are ready at equal cost, the extra action goes first. Batch must commit that Disengage before the action's move, which is out of the current Action, Bonus, Extra order (batch.rs ~691).
  - Cap `teleport_value`'s avoided attacks when a free Disengage was available.
  - Hasten is Recharge 5–6, so include its resource cost, and check that `pick_best`'s limited-against-at-will rule doesn't block it.
  - The reason line names the ability that avoided the attacks, e.g. "steps out of melee, nimble-escape avoids 4.2 opportunity damage".
  - Test the retry path: when the action plan fails after Disengage is spent, the recompute sees the flag and a penalty of 0.
- Findings (batch.rs ~352): stop calling `no_provoke` and `disengage` abilities "Not modeled in phase 1".
- Docs:
  - ADR 0005 gains `disengage`. In ADR 0012, "The policy never plans the Disengage action" (line 11) and "and never Disengage" (line 13) both become "only to step out without provoking".
  - The live-mode reference adds three lines: a bonus Disengage avoids Opportunity Attacks without spending the action (live-mode.md ~68); when the DM picks Hide or Dash, an override sets `bonus_action_used`; Hasten's Dash half is the DM's call.
  - Add a CHANGELOG line.
- Replay: test that a live encounter saved on 0.1.24 with a goblin refolds to its recorded state hashes, since its initial state holds the old unmodeled Nimble Escape.
- Versions: bump the engine patch, regenerate fixtures and the curve with `bench baseline generate`, and check the calibration row. Expect only the assassin's cells to move among the 11 fixtures. If a goblin, spy or melee-only fixture changes, find out why before committing.

## Resolution

Built on 2026-11-06 in auto mode, commit 5572ece. A haiku auditor approved after three rounds.

**Engine and live mode.** `Effect::Disengage` is an additive variant at schema version 1. It sets `action_economy.disengaged`, so Opportunity Attack eligibility is unchanged. `bench action goblin-warrior nimble-escape` runs with no target, spends the bonus action and not the action, and logs `goblin-warrior uses Nimble Escape to Disengage`.

**Import.** The four wordings match on bonus actions only, on 11 fixtures. Each becomes `disengage` plus an agent clause for its other actions ("The goblin may take the Hide action instead of Disengage.", "The golem also takes the Dash action."), so it is partly modeled. Each one gets the review note "Disengage pattern". Hasten keeps its Recharge 5–6, and Prowl stays unmodeled. Like the 207 other partly modeled fixture abilities, the clause needs a standing assumption in batch. Resolving it leaves the Disengage running.

**Policy v4.** An ability whose effects are all `disengage` has no plan of its own. Only action plans Disengage first. When the plan steps out, the cheapest ready bonus Disengage or an extra action that allows it is taken if its cost is below `survival × penalty`. The cost is the best other bonus plan plus a limited ability's resource cost, or the best extra-action plan, both scored on the state before the action. Ties go to the extra action. The reason reads `steps out of melee, cunning-action avoids 5.3 opportunity damage` or `haste's extra action avoids …`. `choose(Bonus)` returns nothing once the bonus action is spent. A teleport in an action plan is worth no more than a ready Disengage. Batch commits the Disengage before the move. A plan that fails afterwards still logs it, and the recompute steps out with a penalty of 0. `pick_best` never sees a Disengage, so its limited-use rule can't block Hasten.

**Findings.** `no_provoke` traits get no finding and no "Skipped mechanic warning". Disengage abilities get the usual agent-clause or assumed finding.

**Versions and docs.** Engine 0.1.25, `POLICY_VERSION` 4. ADR 0005 gains the effect, ADR 0008 the import shape (its bullet no longer gives a count), and ADR 0012 the "only to step out without provoking" wording plus a v4 Disengage paragraph. The live-mode reference has the three lines, the batch-mode reference says v4, and the report's method text and the CHANGELOG are updated. A save recorded with the real 0.1.24 binary (`bench-core/tests/data/engine-0.1.24-state.json`, holding the old unmodeled Nimble Escape) refolds to its hashes.

**Curve.** It was regenerated. The assassin moves 19 cells. The vampire familiar also moves 10 cells, which the decision did not expect: its Umbral Dagger is melee or thrown, so it now Disengages away from the creature it is engaged with and throws at another target without the close-combat Disadvantage. Fight logs confirm it, and it matches the decision's rule for ranged attackers. The other nine fixtures play as before and change only in `ignored_abilities`. The calibration row matches.

Reports: [requirements](../reports/099-requirements.md), [implementation](../reports/099-implementation.md).
