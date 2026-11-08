---
id: 097
title: Batch value of a target teleport
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

A teleport with `"subject": "target"` moves an opponent away and policy v3 scores it 0, as ticket 045 left pulls. It is close to a push to far, so it may reuse the push factors. How should engagement clearing without a band change be valued, and does it need a new policy version (v4 goes to ticket 099)?

## Notes

- Source: [Build teleport subject](../closed/078-build-teleport-subject.md).
- Related: [094](../closed/094-batch-value-of-debuffs.md) kept policy v4 because no curve input scores its terms. This ticket decides its own version by the same rule.
- A policy change regenerates the baseline curve once. 094 kept v4, so this ticket bumps the version only if its own scoring moves a baseline cell.
- Skills: `grilling`, `domain-modeling`. If the answer is code, the build is a follow-up `task` ticket.

## Resolution

Resolved in auto mode on 2026-11-08. The main thread asked the questions and a grilling subagent made the decisions. It accepted every recommendation but one, its own range condition, which the audit overturned (decision 3).

Facts, checked against the code:

- The engine clears every engagement the subject holds, on both sides, at any range, and doesn't touch range bands. A push clears only the pair with its source and changes only that pair's band.
- No SRD fixture, standard party entry or effects-file spell has `"subject": "target"`, and no SRD monster's text teleports an opponent. Only homebrew and engine tests carry it.
- The ticket's premise, "close to a push to far", doesn't hold in the engine. A 60-foot target teleport leaves the target close by, never at Far.

Decisions:

1. **Price the fight the engine plays.** A target teleport scores as the short case of a push, whatever its range. Moving the target to Far would reverse ADR 0005 for an effect with no carrier, and a factor of 1.0 would reward a separation that never happens. Leaving it at 0 would ignore a real effect.
2. **`threat(target) × 0.25` as control**, the same 0.25 as the push's short case. A smaller factor would be a guess with nothing to calibrate on, and so would a new `policy.*` input.
3. **Two conditions give 0.** A target with any attack that doesn't need engagement (as a push), and a target that holds no engagement, because the teleport then does nothing. The push doesn't get that last check, and changing it is outside this ticket. The push's under-5-feet zero doesn't carry over. The engine has no distance check on a teleport, so any `range` clears the engagements.
4. **One factor on the target's threat.** Allies the teleport frees add nothing, since no plan reads a freed ally's position.
5. **Policy stays v4.** Nothing in the curve carries a target teleport. The build bumps the workspace patch and confirms the curve differs only in `engine_version`. If a cell moves, the version question reopens.
6. **The leftover `Engaged` range entry is its own ticket.** After any teleport, `bench move` walks back in for 0 feet while the policy expects 5. A fix reaches self teleports on four fixtures and how saved encounters refold, so it is [108 Leftover Engaged range after a teleport](../tickets/108-leftover-engaged-range-after-a-teleport.md). That ticket also checks whether its answer changes this price.

ADR 0012 is updated. `CONTEXT.md` doesn't change, since no new term appears. The build is [107 Build target teleport scoring](../tickets/107-build-target-teleport-scoring.md).
