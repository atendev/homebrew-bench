---
id: 004
title: Create build tickets and append them to the map
type: grilling
mode: HITL
status: open
blocked_by: [001, 002, 003]
claimed: false
---

## Question

From the decisions closed on this map, what are the implementation tickets, in what order, and what does each one deliver?

Decide:

- The slices, each small enough for one session and written so that `tdd` can drive it
- The order and the blocking edges between them
- Which fog items from Not yet specified become build tickets, and which stay open

Then create the build tickets in `tickets/` with the next free number, and append them to this map's Frontier.

## Notes

- Grill with `grilling` and `domain-modeling`.
- Build tickets are the output of this ticket. Each one names the skills a build session calls: `tdd`, `codebase-design`, `domain-modeling`, `code-review`, and `writing-for-agents` when it edits a skill.
- Blocked by 001, 002 and 003. Do not start until all three have closed.

## Resolution

<!-- Filled in at close: the decision and the reason for it. -->
