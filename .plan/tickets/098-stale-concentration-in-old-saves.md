---
id: 098
title: Stale concentration in old saves
type: grilling
mode: HITL
status: open
blocked_by: []
claimed: false
---

## Question

A pre-0.1.18 save can hold a down or incapacitated creature with a stale concentration, and an override aimed at it with the same value leaves it. Should loading, or any override, sweep it, given the replay-divergence risk the 082 audit raised?

## Notes

- Source: [Override ends concentration for end state and concentration overrides](../closed/082-override-ends-concentration-for-end-state-and-concentration-overrides.md). The removal half was built in [Removal ends concentration](../closed/083-removal-ends-concentration.md).
- Replay starts from the saved initial state, so a sweep on load could change what old recorded encounters replay to. Check whether any committed save or fixture holds a stale concentration.
- Skills: `grilling`, `domain-modeling`. If the answer is code, the build is a follow-up `task` ticket.

## Resolution

<Filled in at close.>
