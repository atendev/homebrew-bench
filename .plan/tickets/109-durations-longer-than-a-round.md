---
id: 109
title: Durations longer than a round
type: grilling
mode: HITL
status: open
blocked_by: []
claimed: false
---

## Question

`decrement_durations` subtracts 1 from `rounds`, `minutes`, `hours` and `days` alike at the end of every creature's turn, so a `{"hours": 1}` modifier or condition ends with the first turn that closes after it lands. How should these durations count down, and how do encounters saved before the change keep refolding as recorded?

## Notes

- Source: [Build See Invisible](../closed/105-build-see-invisible.md), which stored See Invisibility as `permanent` to avoid this.
- Current data that the answer changes: Charm Person's Charmed (`hours`) and Sunburst's Blinded (`minutes`) in `data/spells/effects.json`. Homebrew may carry any of these units.
- `rounds` also loses 1 per creature's turn rather than per round. Decide whether it is part of this ticket. Four entries in `data/spells/effects.json` use `rounds`.
- A change moves replay hashes for saves that hold such a duration, so it may need a recorded-version gate like `settings.reaction_windows`. Moving batch numbers needs a curve regeneration.
- If the units get a real length, restore See Invisibility's `{"hours": 1}` and drop its duration agent clause.
- Skills: `grilling`, `domain-modeling`. If the answer is code, the build is a follow-up `task` ticket.

## Resolution

<Filled in at close: the decision and the reason for it.>
