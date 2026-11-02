---
id: 046
title: Batch policy takes reactions
type: grilling
mode: HITL
status: open
blocked_by: [044]
claimed: false
---

## Question

When should the batch tactics policy spend a reaction, for both sides? Cover opportunity attacks under the rule ticket 044 sets, stat-block reactions on the six reaction triggers (such as Parry), and reaction spells in the standard party list (such as Shield), and decide how one reaction is chosen when several are open at once.

## Notes

v1 takes no reactions (ADR 0012, `bench-core/src/tactics.rs`). The six reaction triggers come from ADR 0005. A change here bumps the policy version and the baseline curve. Consult `grilling` and `domain-modeling`.

## Resolution
