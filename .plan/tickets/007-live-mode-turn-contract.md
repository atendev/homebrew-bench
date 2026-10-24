---
id: 007
title: Live mode turn contract
type: grilling
mode: HITL
status: open
blocked_by: [004]
claimed: false
---

## Question

In live mode, what does one engine command cover, and who rolls the dice? Does the agent submit one creature action that the engine rolls and resolves, or does it step through attack, damage and saves separately? Can the user roll physical dice and enter the results? How do turns and rounds advance, and how does the agent stay honest to the engine's state while narrating?

## Notes

Graduated from the fog item "the contract between the agent-as-DM and the engine in live mode". The interface is fixed by "How the agent talks to the engine": a one-shot CLI with JSON on stdout, a state file per encounter, a tamper check, and a `bench override` command that requires a reason. Override path syntax belongs to the override and log fog item, not here. "Monster schema and engine state model" fixed one step function that live mode calls once per command, and left this ticket to decide what one step covers and how reaction windows open. Consult `grilling` and `domain-modeling`.

## Resolution
