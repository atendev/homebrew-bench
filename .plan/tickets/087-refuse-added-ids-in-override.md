---
id: 087
title: Refuse added ids in the override path
type: task
mode: AFK
status: open
blocked_by: []
claimed: true
---

## Question

Build the refusal decided in [Adding a combatant by override](../closed/086-adding-a-combatant-by-override.md): an override whose result holds a combatant id the encounter did not have fails whole, with an error naming `bench add`.

## Notes

- Check beside the existing removal check in the `Command::Override` arm of `live.rs` (the `previous` versus `state.combatants` comparison), inside `atomically` so nothing applies on refusal.
- Reproduction: `bench start goblin-warrior party=goblin-boss`, then override `combatants` with the list plus a copy under a new id. Today it succeeds, with no initiative, no turn and no ADD line.
- Test first with `tdd`: the list form refuses, state and log are unchanged, and existing removal and field overrides still pass. Bump the engine patch version and add a CHANGELOG entry. Policy stays v3.
- Check the live-mode reference and ADR 0006 wording say the refusal. Skills: `tdd`, `codebase-design`, `code-review`.

## Resolution

<Filled in at close.>
