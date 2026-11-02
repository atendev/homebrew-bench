---
id: 045
title: Batch policy scores buffs, forced movement and teleports
type: grilling
mode: HITL
status: open
blocked_by: []
claimed: false
---

## Question

How should the batch tactics policy score plans that use buffs (timed modifiers on allies), forced movement, and teleports, which v1 scores at zero? Decide a value for each effect kind in the same units the policy already uses for damage and control, and whether a v3 policy changes any existing SRD fixture's results enough to need a regenerated curve.

## Notes

The policy lives in `bench-core/src/tactics.rs` and ADR 0012. A monster built around these effects reads weaker in batch reports than it plays. Any scoring change bumps the policy version and makes the committed baseline stale until `bench baseline generate` reruns. Consult `grilling` and `domain-modeling`.

## Resolution
