---
id: 003
title: Live spell casting prompts
type: grilling
mode: HITL
status: open
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

<!-- Filled in at close: the decision and the reason for it. -->
