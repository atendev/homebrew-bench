---
id: 088
title: Refuse duplicate combatant ids in the override path
type: task
mode: AFK
status: open
blocked_by: []
claimed: true
---

## Question

Build a refusal for an override whose resulting `combatants` holds the same id twice, such as a `combatants` list with two `goblin-warrior` entries. Today it applies: the 087 added-id check and the removal check both pass because no id is new or missing. The second entry can't be addressed by id, and initiative, engagement and ranges key on the id. Refuse it whole, naming the duplicated id, matching ADR 0006.

## Notes

- Found by the 087 audit. Check beside the added-id check in the `Command::Override` arm of `live.rs`, inside `atomically`.
- Test first with `tdd`: the list form with a duplicate refuses and leaves state unchanged, and same-ids lists and field overrides still apply. Bump the engine patch version and add a CHANGELOG entry. Policy stays v3.
- Check whether ADR 0006 and the live-mode reference need a line. Skills: `tdd`, `codebase-design`, `code-review`.

## Resolution

<Filled in at close.>
