---
id: 044
title: Engine-rolled opportunity attacks
type: grilling
mode: HITL
status: open
blocked_by: []
claimed: false
---

## Question

Should the engine resolve opportunity attacks itself, in batch and in live mode, and if so, how? Decide which attack a creature uses (its first melee attack, a policy pick, or a stat-block field), when the attacker's reaction is spent, how live mode lets an off-turn attack run through `bench action` instead of `bench roll` plus `bench override`, and whether a PC's opportunity attack in ask mode raises a pending prompt.

## Notes

Today a move that breaks engagement only returns an `opportunity_attack` event naming who may attack (`set_band` in `bench-core/src/engine.rs`). `bench action` refuses an ordinary attack off-turn, and the batch runner logs the event and moves on. Both batch tactics reactions and live reaction windows build on this answer. A batch change moves the baseline curve, so note whether the tactics policy version must bump. Consult `grilling` and `domain-modeling`; ADRs 0004 (pending prompts) and 0012 (tactics policy) are the constraints.

## Resolution
