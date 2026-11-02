---
id: 041
title: PC initiative prompts, concentration save stream and standing assumptions
type: task
mode: AFK
status: closed
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

All three gaps are fixed, with tests. Release notes start in `CHANGELOG.md` under Unreleased.

- PC initiative: `StepRequest::RollInitiative` runs initiative through `run_engine`, so with `--pc-dice ask` `bench start` holds round 1 behind a d20 prompt for each PC. In ask mode `initial_snapshot` is taken before initiative, and replay re-runs it from the recorded `respond`. A surprised PC enters two faces and keeps the lower. Under group initiative, PCs sharing a stat block share one prompt. `bench add` of a PC in ask mode prompts the same way: the add is validated by a dry run, and `respond` re-runs it. `InitiativeOptions.pc_dice` is gone, because the engine reads `settings.pc_dice`.
- Concentration stream: `apply_damage` takes the caller's `&mut Stream`, so the save draws after the action's earlier dice and its advance persists. Replays of older encounters diverge from the first concentration save, and old ask-mode PC adds diverge at the add. Both are in the changelog.
- Standing assumptions: `bench override --standing` records `{path, value, reason}` and tags the OVERRIDE line `[standing assumption]`. Event lines whose actor or target is the subject end in `[standing assumption: <path>]`. The subject is the path's leading combatant, or every combatant using `stat_blocks.<id>`. Any later override on the same path replaces the assumption, or drops it when the later override is not standing (user decision). Assumptions on other state paths are recorded but tag no event lines. `LogLine::tagged` carries the tag into replaced public lines.
- `(player dice)` is now per event. `EngineEvent.player_dice` is set from `PlayerDice::all_entered_since(owner)` at each dice-bearing event, so engine-filled dice after a decline are not marked. Heal, temporary hit point and death save dice are not marked yet, so that moved to Not yet specified.
- The batch-log standing-assumption header from ADR 0006 belongs to Batch tactics policy and batch runner.
- Audit: haiku, six rounds, approved on round six.
