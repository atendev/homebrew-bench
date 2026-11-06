---
id: 084
title: Override removal bookkeeping
type: task
mode: AFK
status: open
blocked_by: []
claimed: true
---

## Question

Graduated from Not yet specified, found by the 083 audit. Dropping a creature by override (`combatants.<id>`, `combatants.<index>`, or a new `combatants` list) leaves its id in `initiative_order`, `initiative_rolls`, other creatures' engagements and ranges, and `active_turn`, which `bench remove` cleans up (and the active creature's turn isn't ended first). Decide whether override removal should run the same cleanup as `bench remove` or be refused in favour of `bench remove`, and build it.

## Notes

Consult `tdd` and `code-review`. Look at `remove_combatant` and the override path in `bench-core/src/live.rs`, which ticket 083 made run on a trial copy and detect dropped creatures. The likely answer is to run the same cleanup `bench remove` does, since override already finds the dropped creatures. Cover all three removal forms and the active creature case. Update the live-mode reference and the CHANGELOG under Changed with a Replay compatibility bullet, and bump the workspace version one patch. Override and remove are live only, so `POLICY_VERSION` stays 3 and the curve differs only in `engine_version` if regenerated.

## Resolution

<Filled in at close.>
