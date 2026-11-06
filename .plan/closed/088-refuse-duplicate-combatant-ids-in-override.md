---
id: 088
title: Refuse duplicate combatant ids in the override path
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Build a refusal for an override whose resulting `combatants` holds the same id twice, such as a `combatants` list with two `goblin-warrior` entries. Today it applies: the 087 added-id check and the removal check both pass because no id is new or missing. The second entry can't be addressed by id, and initiative, engagement and ranges key on the id. Refuse it whole, naming the duplicated id, matching ADR 0006.

## Notes

- Found by the 087 audit. Check beside the added-id check in the `Command::Override` arm of `live.rs`, inside `atomically`.
- Test first with `tdd`: the list form with a duplicate refuses and leaves state unchanged, and same-ids lists and field overrides still apply. Bump the engine patch version and add a CHANGELOG entry. Policy stays v3.
- Check whether ADR 0006 and the live-mode reference need a line. Skills: `tdd`, `codebase-design`, `code-review`.

## Resolution

Built on branch `ticket-088-refuse-duplicate-ids-in-override` (commit b7481d7). In the `Command::Override` arm of `live.rs`, inside `atomically` and before the added-id check, an override is refused when some id now appears in `combatants` more than once and more often than before the override, with `<id> is in combatants twice; each combatant needs its own id, so use `bench add` for another one`. Counting against `previous` means a duplicate the override didn't create doesn't block it, so an encounter that already holds one can still take field overrides. Paths: a `combatants` list with an entry repeated and `combatants.<index>` set to a copy of another creature are refused. `combatants.<id>.id` and `combatants.<index>.id` were already refused as id leaves. The test `override_refuses_duplicate_ids` covers both forms with state unchanged, plus a same-ids list, a field override, and a field override on a state with an existing duplicate. ADR 0006 and the live-mode reference state the refusal. Engine 0.1.23, policy stays v3, the curve differs only in `engine_version`.

The first haiku audit found that `start` can already produce duplicate ids (entrants `goblin` twice and `goblin-2` once give two `goblin-2`), which made a whole-list check refuse every override. The check was scoped to duplicates the override creates, and the `start` collision became ticket 089. The second audit approved. Its nit: the message says "twice" even when an override makes a third copy. Left as is.
