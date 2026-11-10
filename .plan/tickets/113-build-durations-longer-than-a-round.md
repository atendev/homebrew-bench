---
id: 113
title: Build durations longer than a round
type: task
mode: AFK
status: open
blocked_by: [109]
claimed: true
---

## Question

Build the decision in [Durations longer than a round](../closed/109-durations-longer-than-a-round.md): minutes, hours and days count against the round clock behind a recorded setting, so saved encounters refold as recorded.

## Notes

- Skills: `implement`, `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- `decrement_durations` in `bench-core/src/engine.rs` (~1985) handles modifiers and conditions in two matches. Leave the `Rounds` branch alone. With the gate on, skip `Minutes`, `Hours` and `Days` there, and expire them against the round clock instead. With it off, keep the current per-turn decrement.
- Length in rounds: minute 10, hour 600, day 14400. Expire when `state.round >= started_round + length`, checked as the round counter advances (and in `finish_turn`). Add `started_round` (serde default, skipped at 0) to `ActiveModifier` and `ConditionInstance`, set where each is applied.
- Gate on a new recorded setting, `settings.long_durations`, as `settings.teleport_near` is (`encounter.rs`, `live.rs`, `batch.rs`, `bench/src/main.rs`). New encounters record it on. A save without it loads false and replays as before. State the legacy branch's removal plan in ADR 0005 (closed effect vocabulary), which carries the duration vocabulary: it stays until a saves-migration ticket drops it.
- Restore See Invisibility's `{"hours": 1}` in `data/spells/effects.json` and drop its agent clause about the length.
- Tests: with the gate on, an `hours` modifier and condition survive several turns; Blinded `minutes: 1` clears after 10 rounds and not before; with the gate off, behavior is byte-identical to before; a save without the setting replays unchanged.
- Run the curve with the gate on and compare cells. Charm Person and Sunburst are the batch-visible users of these units. Regenerate and bump the policy version only if cells move, decided from that diff.
- Bump the engine version and add a changelog entry.

## Resolution

<Filled in at close: what was built and the commit.>
