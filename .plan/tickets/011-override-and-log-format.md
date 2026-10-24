---
id: 011
title: Override mechanics, log format and public view
type: grilling
mode: HITL
status: open
blocked_by: []
claimed: false
---

## Question

What path syntax does `bench override` use to set engine state? How do overrides, user-entered rolls, `bench roll` results, undo reversals, pending prompts and agent-judgment labels read in the log? How does the log read to a human in live mode and in a sampled batch log? What does the optional public view hide, and how are wounded states worded?

## Notes

Inputs are closed: the interface ticket fixed `bench override` with a required reason and a tamper hash on every log entry. The state-model ticket fixed the state fields. The live-mode ticket fixed undo, `bench roll`, roll sources and the events list in `docs/adr/0004-whole-actions-with-pending-prompts.md`. Consult `grilling` and `domain-modeling`.

## Resolution
