---
id: 041
title: PC initiative prompts, concentration save stream and standing assumptions
type: task
mode: AFK
status: open
blocked_by: []
claimed: false
---

## Question

Fix three gaps that ticket 030 left in live mode.

## Notes

- PC initiative under `--pc-dice ask`: `live::start` passes `InitiativeOptions { pc_dice: false }` and `engine::roll_initiative` never reads the flag, so PC initiative d20s are engine-rolled even though the START line says `pc dice ask`. Prompt for them through `PlayerDice` as other PC rolls do, before round 1 starts.
- Concentration save stream reuse: `apply_damage` (`bench-core/src/engine.rs`, concentration branch) builds a fresh `Stream` from `state.rng` while the caller's action stream is still live and not yet written back. The save's d20 reuses a cursor the action already drew from, and the caller's `state.rng = stream.state()` then overwrites the save's advance. The rolls stay deterministic but correlated. Thread the caller's `&mut Stream` into `apply_damage`. Replays of recorded encounters will diverge, so note it in the release notes.
- Standing-assumption tags: the log format calls for standing-assumption tags, but `EncounterState.standing_assumptions` is never written. Decide which command sets one (likely `bench override --rule` with a standing flag, or a new `bench assume`) and tag the log lines it affects.
- Use `tdd`. Each fix needs a failing test first.

## Resolution
