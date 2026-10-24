---
id: 004
title: Monster schema and engine state model
type: grilling
mode: HITL
status: open
blocked_by: [002, 003]
claimed: false
---

## Question

What fields does a homebrew monster have, and what state does the engine track per creature and per fight?

## Notes

Start from the SRD monster shape for stats, defenses, attacks, save actions, usage, and multiattack. The state model covers HP, conditions, concentration, resources, recharge, and action economy.

Decided with the user: the engine does not track position or areas. This is not a full simulator. Still open: how abilities that depend on position get resolved without it. Examples from 005 are area target counts, Pack Tactics, auras, and movement tricks. Options include a fixed assumption per shape, a value the agent sets at intake, or marking them as agent judgment.

## Resolution
