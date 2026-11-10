---
id: 109
title: Durations longer than a round
type: grilling
mode: HITL
status: closed
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

Minutes, hours and days count against the round clock, and `rounds` stays as it is.

- **Length.** One minute is 10 rounds, an hour 600 and a day 14400. A modifier or condition holding one of these units expires when `state.round >= started_round + length`, checked as the round counter advances. The stored value stays `hours: 1`, so players and agents see the unit the spell states. An hour or a day is never reached inside a fight, and Sunburst's Blinded (`minutes: 1`) clears after 10 rounds.
- **Representation.** `ActiveModifier` and `ConditionInstance` gain a `started_round` field, serde default and skipped at 0. With the gate on, `decrement_durations` skips minutes, hours and days.
- **Old saves.** A recorded setting, `settings.long_durations`, gates it as `reaction_windows` and `teleport_near` do. A save without it loads false and keeps the per-turn decrement, so it refolds as recorded. `bench start` and batch set it true.
- **Rounds.** `rounds` still loses 1 at the end of every creature's turn. Moving it to a per-round count changes Shield and Haste play and batch cells, so it is its own fog item under Not yet specified.
- **See Invisibility.** The build restores `{"hours": 1}` and drops the agent clause about its length. It scores 0, so cells should not move; confirm Charm Person's and Sunburst's cells before saying no regeneration is needed.

Decided in auto mode by a grilling subagent over six questions, with no changes to the recommendations. The build is [113 Build durations longer than a round](../tickets/113-build-durations-longer-than-a-round.md).
