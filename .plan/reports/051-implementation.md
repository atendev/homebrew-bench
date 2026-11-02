# Ticket 051 implementation report

Ticket: `.plan/tickets/051-player-dice-marks-for-heal-temp-hp-and-death-saves.md`. Requirements: `.plan/reports/051-requirements.md`. Diff reviewed: `git diff` on branch `ticket-051-player-dice-marks`, uncommitted changes.

## What was built

- `bench-core/src/engine.rs`:
  - `Effect::Heal` reads `all_entered_since(actor_id)` once, right after the heal dice and before the target loop. Every `heal` event and the `revive` event a heal triggers carry that mark.
  - `Effect::TempHp` now emits a `temp_hp` event per target, `<target> gains <N> temporary HP`, marked by the same rule. Before, it changed `temp_hp` and logged nothing.
  - `roll_death_saves` reads `all_entered_since(combatant_id)` after the d20. The `death_save` event is built once from the branch message and sets `roll: Some(roll)` and `player_dice`.
  - Three new tests: entered faces mark heal, revive and temp hp; an engine-filled die leaves them unmarked (including `--pc-dice engine`); death saves carry the d20 and are marked only for an entered face.
- `bench-core/src/live.rs`: `event_line` gets a `temp_hp` arm. In the public view, a monster's temp hp line becomes `<target> gains temporary HP`. New test `public_view_hides_monster_temp_hp_amounts`.
- `CHANGELOG.md`: the "not marked yet" sentence is replaced by one saying heal, revive, temporary hit point and death save events follow the same rule. Two new entries: the `temp_hp` event line (live logs and batch fight logs, results and dice unchanged), and the `[d20 N]` suffix on death save lines.
- `.plan/tickets/051-player-dice-marks-for-heal-temp-hp-and-death-saves.md`: `claimed` changed from `false` to `true`.

## Requirements coverage

- Heal and revive marked, one read for all targets. Met.
- Temp hp event added and marked. Met.
- Death save events marked and carry `roll`. Met.
- All-or-nothing marking. Met. A declined die or a mixed heal leaves the event unmarked, and each dice-bearing event calls `all_entered_since` exactly once.
- Monster heals, monster temp hp and auto mode stay unmarked. Met, because `player_roll_in` already returns the engine die in those cases.
- JSON and log format. Met. The existing `player_dice` field and `skip_serializing_if` are reused, and `event_line` already renders `(player dice)` after `[d20 N]`.
- Batch results unchanged. Met. No dice draw order or `Stream` use changed. Batch fight logs gain `temp_hp` lines, which the changelog says.

## Deviations and extra choices

- The "suffers N death save failure(s) from the hit" event in `apply_damage`, plus `stabilize` and `dies`, stay unmarked. They roll no die of their own.
- No replay-compatibility changelog entry. Replay is unaffected because `game_hash` excludes the log.
- The public view hides the amount of a monster's temp hp, matching how monster heals are redacted. The requirements did not list this, but the new event would otherwise leak it.

## Audit

- Round 1 (haiku) found two problems: the `revive` event a heal triggers was unmarked, and the changelog did not say batch fight logs now carry `temp_hp` lines. Both were fixed.
- Round 2: APPROVE. Non-blocking notes: no CLI-level JSON test for `player_dice`, and the public-view test does not check a marked monster line.

## Verification

- `cargo test --workspace` passes (138 bench-core lib tests).
- Clippy shows only warnings that were already in the repository.

## Left for the user

- Decide whether to add a CLI-level JSON test for `player_dice` and a marked monster line in the public-view test.
- Commit the change. The ticket's Resolution section is still empty.
