---
id: 089
title: Refuse colliding combatant ids at start
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Build a fix for `start` giving two combatants the same id. It suffixes an id with `-<n>` only when a stat block id occurs more than once, and never checks the result against other ids. Entrants `goblin` twice and `goblin-2` once give two combatants with id `goblin-2`, and `start` accepts it. The second can't be addressed by id, and initiative, engagement and ranges key on the id. Decide between refusing the encounter with an error naming the id and picking the next free suffix, then build it.

## Notes

- Found by the 088 audit. The suffixing is in `start` in `bench-core/src/live.rs` (the `counts` and `seen` maps). Check whether `bench add`'s id choice (`add_combatant`) can collide the same way, and whether batch builds ids through the same path.
- Test first with `tdd`. Bump the engine patch version and add a CHANGELOG entry, with a replay line if recorded encounters can diverge. Policy stays v3.
- Skills: `tdd`, `codebase-design`, `code-review`.

## Resolution

Decision: pick the next free number, not refuse. `bench add` already does this, and refusing would make the user rename a stat block to get past a collision the engine can avoid by itself.

Built on branch `ticket-089-refuse-colliding-ids-at-start` (commit eed1ccf). In `start` in `live.rs`, the ids of stat blocks used once go into a `taken` set before numbering starts. A repeated stat block's `-<n>` counter skips any number already in `taken`, and every id it hands out joins the set. Two `goblin-warrior` entrants and one `goblin-warrior-2` now start as `goblin-warrior-1`, `goblin-warrior-3` and `goblin-warrior-2`, in any entrant order. Names follow the number ("Goblin Warrior 3"). The test `start_suffixes_past_ids_already_taken` covers it, plus initiative and ranges for all four combatants.

Checked: `add_combatant` can't collide, because it searches live ids from `-2` for a free one. Batch calls `live::start` with one monster and distinct party blocks, so no batch id changes. Replay and undo fold from `initial_snapshot`, set at the end of `start`, so recorded encounters keep their ids and the CHANGELOG has no replay line. The live-mode reference states the numbering rule. Engine 0.1.24, policy stays v3, the curve differs only in `engine_version`. Audit (haiku): APPROVE, with one wording nit in live-mode.md that was fixed.
