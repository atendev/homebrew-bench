---
id: 012
title: Randomness, seeds and replay
type: grilling
mode: HITL
status: open
blocked_by: []
claimed: false
---

## Question

How does the engine produce and reproduce rolls? Is there a seed per encounter and per batch? Does a batch report record its seed so a sampled fight can be replayed? How does replay treat user-entered dice, `bench roll`, overrides and undo? Where does the random state live in the state file, and which RNG algorithm is fixed so results match across platforms and engine versions?

## Notes

Live encounters already persist as state files, so this is about reproducing rolls, not resuming. The state-model ticket listed randomness state in the encounter file and left its shape here. The live-mode ticket added user-entered dice and `bench roll`. Consult `grilling` and `domain-modeling`.

## Resolution
