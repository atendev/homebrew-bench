---
id: 087
title: Refuse added ids in the override path
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Build the refusal decided in [Adding a combatant by override](../closed/086-adding-a-combatant-by-override.md): an override whose result holds a combatant id the encounter did not have fails whole, with an error naming `bench add`.

## Notes

- Check beside the existing removal check in the `Command::Override` arm of `live.rs` (the `previous` versus `state.combatants` comparison), inside `atomically` so nothing applies on refusal.
- Reproduction: `bench start goblin-warrior party=goblin-boss`, then override `combatants` with the list plus a copy under a new id. Today it succeeds, with no initiative, no turn and no ADD line.
- Test first with `tdd`: the list form refuses, state and log are unchanged, and existing removal and field overrides still pass. Bump the engine patch version and add a CHANGELOG entry. Policy stays v3.
- Check the live-mode reference and ADR 0006 wording say the refusal. Skills: `tdd`, `codebase-design`, `code-review`.

## Resolution

Built on branch `ticket-087-refuse-added-ids-in-override` (commit 4502693). In the `Command::Override` arm of `live.rs`, inside `atomically`, an override whose resulting `combatants` holds an id that `previous` lacked is refused with `<id> is not in the encounter; an override can't add a combatant, use `bench add`, which rolls its initiative`. It runs before the removal checks, so a rename through the list form reports the add. Paths: a `combatants` list with a new entry, a list with an entry renamed, and `combatants.<index>` set to an entry with a new id are all refused (`combatants.<id>.id` was already refused as an id leaf, and an index past the end already fails). The test `override_refuses_added_ids` covers the three forms with state unchanged, plus a same-ids list and a field override that still apply and replay. ADR 0006 and the live-mode reference state the refusal. Engine 0.1.22, policy stays v3, the curve differs only in `engine_version`. The haiku audit approved. It noted that a list with a duplicated existing id is still accepted, which became ticket 088.
