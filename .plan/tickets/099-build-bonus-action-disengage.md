---
id: 099
title: Build bonus-action Disengage
type: task
mode: AFK
status: open
blocked_by: [090]
claimed: true
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

<Filled in at close.>
