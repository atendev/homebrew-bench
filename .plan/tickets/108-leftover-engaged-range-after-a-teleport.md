---
id: 108
title: Leftover Engaged range after a teleport
type: grilling
mode: HITL
status: open
blocked_by: []
claimed: false
---

## Question

A teleport clears engagements but leaves the pair's `ranges` entry at `Engaged`. `bench move` then walks back in for 0 feet, while the policy's `band` reads the same pair as Near and plans 5 feet. Should the engine write Near into those entries, should `band` and `move` read the same state some other way, and what does either do to saved encounters and the curve?

## Notes

- Source: [Batch value of a target teleport](../closed/097-batch-value-of-a-target-teleport.md). Reproduced on 0.1.25 with a throwaway test: after a `target` teleport the fighter's `ranges` held `{"blinker": Engaged}` with an empty engagement list, and `move_combatant` to Engaged with 0 feet succeeded.
- `Effect::Teleport` in `bench-core/src/engine.rs` (~694) clears engagements only. `move_combatant` prices from the `ranges` entry (~1316), defaulting a missing entry to `Engaged`. `band` in `bench-core/src/tactics.rs` (~511) checks engagement first and reads a leftover `Engaged` as Near. `bench-core/src/live.rs` (~381) also reads `ranges`.
- ADR 0005 says a teleport leaves range bands unchanged, so a fix that writes Near has to square with it.
- Reach: self teleports take the same path, and four fixtures have them (blink dog, marilith, balor, nalfeshnee). A fix can change curve cells and how saved encounters refold, so ask whether it needs a recorded-version gate like `settings.reaction_windows` and a policy bump.
- ADR 0012 prices a `target` teleport at the short push factor. Check whether the answer here changes that price.
- Skills: `grilling`, `domain-modeling`. If the answer is code, the build is a follow-up `task` ticket.

## Resolution

<Filled in at close: the decision and the reason for it.>
