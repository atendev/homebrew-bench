---
id: 048
title: Live legendary actions
type: grilling
mode: HITL
status: open
blocked_by: []
claimed: false
---

## Question

How does live mode run legendary actions? Decide the command shape (a new `bench legendary` or `bench action` accepting legendary ids off-turn), when the engine allows one (after another creature's turn ends), how `legendary_uses_per_round` is spent and reset, and whether the engine prompts the agent when a window opens.

## Notes

Stat blocks already carry `legendary_actions` and `legendary_uses_per_round`. The skill currently resolves them with `bench roll` and `bench override`. Check how the batch runner uses legendary actions so both modes share one rule. Consult `grilling` and `domain-modeling`.

## Resolution
