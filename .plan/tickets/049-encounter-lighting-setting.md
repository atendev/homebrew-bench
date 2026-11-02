---
id: 049
title: Encounter lighting setting
type: grilling
mode: HITL
status: open
blocked_by: []
claimed: false
---

## Question

Where does the encounter's lighting come from, and how is it set? Roll modifiers name a `Lighting` value (`bench-core/src/effect.rs`), but no live command sets one. Decide whether lighting is one value per encounter or per combatant, its default, the flag on `bench start` and `bench batch`, whether `bench override` can change it mid-fight, and how the batch report states it as an assumption.

## Notes

ADR 0005 introduced roll modifiers with a `lighting` setting. Darkvision and Sunlight Sensitivity are the common SRD cases. Consult `grilling` and `domain-modeling`.

## Resolution
