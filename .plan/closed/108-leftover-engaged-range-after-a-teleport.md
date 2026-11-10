---
id: 108
title: Leftover Engaged range after a teleport
type: grilling
mode: HITL
status: closed
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

A teleport writes Near into the `ranges` entries of every pair it frees, and the engine state is the fix: `band`, `move`, saved JSON and logs then agree, and no reader gets a second rule. The pair set is captured before engagements clear, as the union of the subject's list and every list that holds the subject, and both sides' entries change. Other bands stay.

- ADR 0005 is amended: gated, an engaged pair lands at Near (a modeling choice, since the engine doesn't model where the subject lands); legacy saves keep bands unchanged.
- The change sits behind a recorded setting like `settings.reaction_windows`. New encounters record it on, saves without it refold as recorded. The build compares curve cells with the setting on, and regenerates and bumps the policy version only if they move.
- ADR 0012's price stays `threat(target) x 0.25`. It is a constant, not derived from re-engagement cost, so only the paragraph's wording changes.
- A missing `ranges` entry (Engaged in `move`, Near in `band`) stays out of scope. Setup fills every cross-side pair, so it only arises for later additions.
- `live.rs` is unaffected: it only prints starting bands before any teleport.
- Build: [Build Near after a teleport](../tickets/112-build-near-after-teleport.md).
