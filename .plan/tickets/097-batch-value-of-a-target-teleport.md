---
id: 097
title: Batch value of a target teleport
type: grilling
mode: HITL
status: open
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

<Filled in at close.>
