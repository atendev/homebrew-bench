# Ticket 051 requirements report

Ticket: `.plan/tickets/051-player-dice-marks-for-heal-temp-hp-and-death-saves.md`. Rule source: `.plan/closed/041-pc-initiative-prompts-and-concentration-stream.md`. Log format: `docs/adr/0006-override-paths-and-log-format.md`.

## Goal

In ask mode (`--pc-dice ask`), mark heal, temporary hit point and death save events `(player dice)` when every die behind the event came from a face the user entered. Attack, damage and initiative events already work this way. These three kinds go through the pending prompt, but their events never set `player_dice`.

## Acceptance criteria

- [ ] Heal events (`kind: "heal"`, and the `revive` event a heal can trigger) set `player_dice` from `PlayerDice::all_entered_since(actor)`. One heal roll covers every target, so every target's event gets the same mark (read it once before the target loop, as `Effect::Damage` does).
- [ ] Temporary hit point effects emit an event. Today `Effect::TempHp` changes `temp_hp` and emits nothing, so there is nothing to mark. Add one event per target, with the amount in its message, and mark it by the same rule.
- [ ] Death save events set `player_dice` from `all_entered_since(combatant_id)`: success, failure, natural 1, natural 20. Set `roll: Some(natural)` on them so the log shows `[d20 N]`, as attack and initiative lines do. The follow-on `stabilize` and `dies` events carry no die of their own and stay unmarked.
- [ ] Marked only when every die was entered. A declined prompt (engine fills the die) leaves the event unmarked. A heal such as `2d8+4` with one entered face and one engine face is unmarked.
- [ ] Not marked: the "suffers N death save failure(s) from the hit" event in `apply_damage`. It rolls no die. The `damage` event for the hit keeps its own mark.
- [ ] Not marked: monster heals and temp hp, and any roll in `--pc-dice auto`. `player_roll_in` already returns the engine die for non-PCs and for non-ask mode, so these stay unmarked.
- [ ] JSON: marked events carry `"player_dice": true`. Unmarked events omit the field (existing `skip_serializing_if`). No new field is needed for the flag.
- [ ] Log: a marked line ends in ` (player dice)` after any `[d20 N]` suffix, from `event_line` in `live.rs`. Public view (`bench log --public`) keeps the suffix, and monster heals still use the replaced wounded-state line.
- [ ] `CHANGELOG.md` (Unreleased, the `(player dice)` entry): delete the sentence "Heal, temporary hit point and death save dice are not marked yet." and state that these events are now marked under the same rule. Note the new temporary hit point event line.
- [ ] Tests first (`tdd`): ask-mode heal, temp hp and death save with an entered face are marked. A declined die is not. A mixed event is not. Auto mode is not.

## Constraints

- ADR 0006: `(player dice)` is an inline marker, not a glyph. Plain-text log stays one line per event, uppercase tags untouched. Batch logs contain no player dice.
- Ticket 041 rule: per-event, all-or-nothing. Call `all_entered_since` exactly once per dice-bearing event, because it clears the owner's counter. A call that is skipped lets the next event inherit the mark.
- Order of calls matters. Read the mark right after the dice for that event, before the next event rolls.
- Batch mode is unaffected: no prompts, `player_dice` stays false, existing batch output and the replay of recorded encounters do not change, apart from the new temp hp event line in logs that include such effects. Check that this does not alter batch results or hashes, and mention it in the changelog if it does.
- Do not change the dice draw order or `Stream` use. Only add reads of the mark and the `roll` field.

## Code locations

- `bench-core/src/engine.rs`: `EngineEvent` (line 46, `player_dice`), `Effect::Heal` (~380), `Effect::TempHp` (~397), `roll_death_saves` (~1060, uses `player_roll`), `apply_damage` down-hit branch (~1223), `player_roll_in` (~1536), `event()` helper (~1570, always `player_dice: false`). Model on `Effect::Damage` (~335) and the attack arm (~306).
- `bench-core/src/encounter.rs`: `PlayerDice::take` and `all_entered_since` (~290).
- `bench-core/src/live.rs`: `event_line` (~740) renders `[d20 N]` and `(player dice)` and handles public redaction for monster heals.
- `CHANGELOG.md`: line 34.
- Tests: engine tests near `engine.rs` ~2193 (damage mark), ~2505 to ~2575 (death saves, heal); `live.rs` ~1176 (declined initiative mark); `bench/tests/cli.rs` for the JSON field.
