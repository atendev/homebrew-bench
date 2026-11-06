---
id: 084
title: Override removal bookkeeping
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Graduated from Not yet specified, found by the 083 audit. Dropping a creature by override (`combatants.<id>`, `combatants.<index>`, or a new `combatants` list) leaves its id in `initiative_order`, `initiative_rolls`, other creatures' engagements and ranges, and `active_turn`, which `bench remove` cleans up (and the active creature's turn isn't ended first). Decide whether override removal should run the same cleanup as `bench remove` or be refused in favour of `bench remove`, and build it.

## Notes

Consult `tdd` and `code-review`. Look at `remove_combatant` and the override path in `bench-core/src/live.rs`, which ticket 083 made run on a trial copy and detect dropped creatures. The likely answer is to run the same cleanup `bench remove` does, since override already finds the dropped creatures. Cover all three removal forms and the active creature case. Update the live-mode reference and the CHANGELOG under Changed with a Replay compatibility bullet, and bump the workspace version one patch. Override and remove are live only, so `POLICY_VERSION` stays 3 and the curve differs only in `engine_version` if regenerated.

## Resolution

Override removal runs the same cleanup as `bench remove`, through a shared `forget_combatant`: the creature leaves `initiative_order`, `initiative_rolls`, `active_turn`, and other creatures' engagements and ranges. All three forms work: `combatants.<id>`, `combatants.<index>`, and a new `combatants` list.

The override now applies first. Each creature it dropped is put back for a moment from a pre-apply snapshot so `engine::end_concentration_on_leaving` can end what it held, and is then removed and forgotten. This fixes a 0.1.20 bug: the bearers' entries in a new `combatants` list used to put back the conditions the leaving creature's spell held.

Two override removals are refused atomically. Dropping the creature that holds the active turn returns `Invalid` with a pointer to `bench remove`, which ends the turn first. Removing anyone while a prompt is pending returns `Pending`, as `bench remove` already did. Ending the turn inside the override was tried and dropped: in the list form it overwrote survivor state, and it ran turns for creatures removed alongside.

Engine 0.1.21, policy stays v3, the curve differs only in `engine_version`. Three haiku audit rounds; the third approved.
