---
id: 083
title: Removal ends concentration
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Graduated from Not yet specified, found by the 082 audit. `bench remove` and an override that clears `combatants.<id>` (or `combatants.<index>`) drop a concentrating creature without ending what it held on others, so its conditions and timed modifiers stay on their bearers with a source that no longer exists. Decide whether removal ends the creature's concentration before it deletes, and build it.

## Notes

Consult `tdd` and `code-review`. Look at `remove_combatant` and the override cleanup in `bench-core/src/live.rs` (ticket 082's before-and-after check) and `engine::end_concentration`. A removed creature is gone from the fight, so the likely answer is yes, through the same `end_concentration` path and log line as 082. Cover `bench remove`, clearing `combatants.<id>`, and clearing by index. The stale pre-0.1.18 save case from the same fog item stays in fog. Update the live-mode reference and the CHANGELOG under Changed with a Replay compatibility bullet, and bump the workspace version one patch. Override and remove are live only, so `POLICY_VERSION` stays 3 and the curve differs only in `engine_version` if regenerated.

## Resolution

Built. A creature that leaves the encounter ends what it holds first. `bench remove <id>` and an override that drops a creature from `combatants` (clearing `combatants.<id>` or `combatants.<index>`, or setting `combatants` without it) call the new `engine::end_concentration_on_leaving` before the creature is deleted. A creature that still concentrates loses it through `break_concentration`, which ends the conditions and timed modifiers the spell held, fires their `on_end` and logs `<id> loses concentration`. A creature whose `concentration` was already cleared by override still ends the concentration-duration effects it sources, logged as `<id> leaves; the effects it held by concentration end`. A trial copy of the override finds which creatures it drops. Override and remove now run on a copy that is saved only if the command succeeds, so a refused command changes nothing. A creature that clears its `concentration` and stays in the encounter is still the agent's job.

Engine 0.1.20, `POLICY_VERSION` stays 3. The regenerated `data/baseline/curve.json` differs only in `engine_version`. Test: `removal_ending_concentration` (all four removal forms, a Haste-style modifier with an `on_end`, replay, undo, a cleared field, a refused override). Workspace tests pass. CHANGELOG under Changed plus a Replay compatibility bullet, and the live-mode reference names the removal forms.

Audit: haiku, four rounds (the user said to audit until approval). Round 1: no atomicity, no replay or undo test, timed modifiers untested, docs disagreed on paths. Round 2: the Replay compatibility bullet left out replacing `combatants`. Round 3: a creature whose `concentration` was cleared by hand left its effects behind on removal. Round 4 approved. The audit noted that override removal leaves the creature's id in `initiative_order`, `initiative_rolls`, engagements and `active_turn`, which `bench remove` cleans up. That predates this ticket and went to fog. The stale pre-0.1.18 save case stays in fog. No CI runs on branch pushes (`release.yml` runs on pull requests and tags only), so nothing was pushed.

