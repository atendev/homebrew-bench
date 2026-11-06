---
id: 083
title: Removal ends concentration
type: task
mode: AFK
status: open
blocked_by: []
claimed: true
---

## Question

Graduated from Not yet specified, found by the 082 audit. `bench remove` and an override that clears `combatants.<id>` (or `combatants.<index>`) drop a concentrating creature without ending what it held on others, so its conditions and timed modifiers stay on their bearers with a source that no longer exists. Decide whether removal ends the creature's concentration before it deletes, and build it.

## Notes

Consult `tdd` and `code-review`. Look at `remove_combatant` and the override cleanup in `bench-core/src/live.rs` (ticket 082's before-and-after check) and `engine::end_concentration`. A removed creature is gone from the fight, so the likely answer is yes, through the same `end_concentration` path and log line as 082. Cover `bench remove`, clearing `combatants.<id>`, and clearing by index. The stale pre-0.1.18 save case from the same fog item stays in fog. Update the live-mode reference and the CHANGELOG under Changed with a Replay compatibility bullet, and bump the workspace version one patch. Override and remove are live only, so `POLICY_VERSION` stays 3 and the curve differs only in `engine_version` if regenerated.

## Resolution

