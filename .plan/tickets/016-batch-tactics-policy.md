---
id: 016
title: Batch tactics policy
type: grilling
mode: HITL
status: open
blocked_by: [013, 014]
claimed: false
---

## Question

What built-in policy picks actions for both sides in `bench batch`? Cover target selection, action choice, use of recharge and limited-use abilities, legendary actions, movement between range bands, the default area target count by area size, and when party members heal or use consumables.

## Notes

Batch is one engine call, so the agent does not choose per-turn actions. The step-engine ADR (`docs/adr/0003-step-engine-with-abstract-engagement.md`) fixed the band movement costs and left the policy open. A standing assumption can override the policy's defaults. Consult `grilling` and `domain-modeling`.

## Resolution
