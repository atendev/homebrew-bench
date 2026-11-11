---
id: 002
title: Homebrew spell intake rules
type: grilling
mode: HITL
status: open
blocked_by: []
claimed: false
---

## Question

When a user pastes a homebrew spell, what does the agent convert, what does it flag, what does it refuse, and how does the human review the result before any run?

Decide:

- Which fields of the effects entry the agent fills from the text, and which it leaves for the human
- What counts as ambiguous wording that must be flagged, and what is refused outright
- The form the review takes, and whether the human can edit the entry or only approve it
- How an entry relates to `data/spells/effects.json`, which is keyed by SRD id

## Notes

- Grill with `grilling` and `domain-modeling`.
- Read [ADR 0005](../../docs/adr/0005-closed-effect-vocabulary-with-agent-clauses.md): homebrew abilities never get a default, and batch refuses until each has a resolution. Keep that rule.
- Read [ADR 0014](../../docs/adr/0014-spell-effects-file-authorship.md). It rejected LLM-authored entries for SRD spells because of churn. Decide how homebrew entries differ.
- Settled earlier: intake is included, with human review before any run.

## Resolution

<!-- Filled in at close: the decision and the reason for it. -->
