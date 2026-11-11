---
id: 003
title: Live spell casting prompts
type: grilling
mode: HITL
status: closed
blocked_by: [002]
claimed: false
---

## Question

In live mode, how does a spell reach the table when either side casts it?

Decide:

- How the user's party casts a homebrew spell: which pending prompt asks for the target, slot level and upcast, and what the user sees before they answer
- How the monster side casts it: which stat block action names the spell, and how the agent chooses it
- What the log records for a cast, a countered cast, and a spell that fails its save

## Notes

- Grill with `grilling` and `domain-modeling`.
- Read [ADR 0004](../../docs/adr/0004-whole-actions-with-pending-prompts.md) for pending prompts, and [ADR 0018](../../docs/adr/0018-reaction-windows-hold-the-triggering-command.md) and [ADR 0019](../../docs/adr/0019-counterspell-through-a-spell-cast-window.md) for counters.
- Blocked by 002 because a cast needs an entry to read.
- Settled earlier: both sides can cast the spell.

## Resolution

Recorded in [ADR 0022](../../docs/adr/0022-live-spell-casts-reuse-bench-action.md). The glossary gains **Cast card**.

- **Party cast.** No new command and no new prompt. The PC's stat block gets an action with a `cast` effect naming the homebrew id, added by the agent and shown to the user. The user casts it with `bench action ... --slot N`. ADR 0004 keeps targets and slot on the command, and the agent asks in chat.
- **What the user sees.** A new read-only `bench spell <id> [--caster <id>] [--slot N]` cast card: the entry resolved at that cast level, the caster's save DC, attack bonus and slots, every agent clause verbatim, and how many target ids the command takes.
- **Monster cast.** The stat block gets an action with a `cast` effect and a fixed `slot_level`. The agent picks it like any ability. No spell menu.
- **Start-time checks.** The cast's section must match the entry's `casting_time`, `long` is refused, a leveled save or attack spell needs a `spellcasting` block, and a missing `--spell` file warns and leaves an `unmodeled_cast`. Spells are fixed at `bench start`. A `long` spell is refused in live mode.
- **Log.** `cast`, then the window closes, then `agent_clause` and `upcast_clause` events for the entry, then saves. A countered cast logs `spell_countered`, emits no clauses, spends the action and not the slot. A failed save is the existing `save` event followed by the entry's effect events. A monster's clauses are hidden from the public view. No new event kinds.
- **Reaction spells.** No window offers them and a cast made as a reaction opens none (ADR 0019). They are cast out of turn with `bench action`. A homebrew counter is a `reactions` entry with the `spell_cast` trigger.
- **Surfaced.** SRD agent-clause emission goes on the map as a new question in Not yet specified.
