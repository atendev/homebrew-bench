---
id: 005
title: Monster schema and engine state model
type: grilling
mode: HITL
status: open
blocked_by: [001, 002, 003]
claimed: false
---

## Question

What fields does a homebrew monster have, and what state does the engine track per creature and per fight? How does the engine advance a fight: whole-fight simulation, one action at a time, or both?

## Notes

The state model has to support both modes. Consider HP, conditions, concentration, resources, recharge, action economy, and whether state can be read and resumed between steps. Also open: whether the engine tracks position, and how position-dependent abilities (area target counts, Pack Tactics, auras) get resolved either way.

## Resolution
