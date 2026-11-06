---
id: 089
title: Refuse colliding combatant ids at start
type: task
mode: AFK
status: open
blocked_by: []
claimed: true
---

## Question

Build a fix for `start` giving two combatants the same id. It suffixes an id with `-<n>` only when a stat block id occurs more than once, and never checks the result against other ids. Entrants `goblin` twice and `goblin-2` once give two combatants with id `goblin-2`, and `start` accepts it. The second can't be addressed by id, and initiative, engagement and ranges key on the id. Decide between refusing the encounter with an error naming the id and picking the next free suffix, then build it.

## Notes

- Found by the 088 audit. The suffixing is in `start` in `bench-core/src/live.rs` (the `counts` and `seen` maps). Check whether `bench add`'s id choice (`add_combatant`) can collide the same way, and whether batch builds ids through the same path.
- Test first with `tdd`. Bump the engine patch version and add a CHANGELOG entry, with a replay line if recorded encounters can diverge. Policy stays v3.
- Skills: `tdd`, `codebase-design`, `code-review`.

## Resolution

<Filled in at close.>
