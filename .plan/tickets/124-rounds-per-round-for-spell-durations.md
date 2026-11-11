---
id: 124
title: Rounds per round for spell durations
type: grilling
mode: HITL
status: open
blocked_by: []
claimed: false
---

## Question

`rounds` durations lose one per creature's turn, so Shield and Haste last one turn. Should spell durations count per round, behind a recorded setting as `long_durations` does, and what does that do to batch cells and spell lift?

## Notes

- Grill with `grilling` and `domain-modeling`.
- Background: the archived map's Rounds per round entry ([archived map](../archive/homebrew-bench-map.md)) and [Durations longer than a round](../closed/109-durations-longer-than-a-round.md), which left the `Rounds` branch alone.
- Decide before the spell baseline is committed. A change afterward moves Shield and Haste lift and forces a regenerate.
- If the answer is a change, create its build ticket and make it block [Spell baseline](125-spell-baseline.md).

## Resolution

<!-- Filled in at close. -->
