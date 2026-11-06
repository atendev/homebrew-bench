---
id: 096
title: Plan-failure warnings split per level
type: grilling
mode: HITL
status: open
blocked_by: []
claimed: false
---

## Question

Batch keys a plan failure on the combatant's name and the error text, so one fault in a standard-party PC repeats once per party level, and errors that name a combatant id split further. Should the key use the stat block's archetype and strip ids from the text?

## Notes

- Source: [Batch warns on quiet plan failures](../closed/070-batch-warns-on-quiet-plan-failures.md).
- Reproduce on the current engine first: run a batch with a deliberately faulty standard-party entry and count the warnings.
- Check whether the report format needs a count per warning instead of a repeat.
- Skills: `grilling`, `domain-modeling`. If the answer is code, the build is a follow-up `task` ticket.

## Resolution

<Filled in at close.>
