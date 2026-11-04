---
id: 069
title: Build saving-throw condition sources
type: task
mode: AFK
status: open
blocked_by: [060]
claimed: false
---

## Question

Make saving throws follow the 2024 conditions. Paralyzed (PHB p. 370), Petrified (p. 371), Stunned (p. 375) and Unconscious (p. 376) automatically fail Strength and Dexterity saving throws, and Restrained (p. 372) has Disadvantage on Dexterity saving throws. `resolve_save` in `bench-core/src/engine.rs` applies none of them.

## Notes

Surfaced by [Align attack-roll condition sources with the 2024 conditions](../closed/067-align-attack-roll-condition-sources-with-2024-rules.md), which left saves out. It waits on [Build encounter lighting](../closed/060-build-encounter-lighting.md), because 060 gives `resolve_save` a disposition through `combined_disposition`.

Engine (`bench-core/src/engine.rs`):

- Auto-fail: before any die is drawn, if the save's ability is Strength or Dexterity and the target has Paralyzed, Petrified, Stunned, or counts as Unconscious (ticket [068](068-build-attack-roll-condition-sources.md)'s helper: the Unconscious condition, or `end_state` Unconscious or Stable), the save fails. It draws no d20, and in ask mode it prompts no player. The event says why: `mia fails the save automatically (paralyzed, Dexterity)`, with `roll` and `total` left empty. The failure's effects (`on_failure`, full damage) apply as for any failed save.
- Restrained: a Dexterity save by a Restrained target adds Disadvantage as one more source in `save_disposition`, the list 060 passes to `combined_disposition`. Don't write a second helper. A Restrained target with an Advantage source from lighting or a modifier rolls straight. Strength saves are untouched.
- Timed modifiers: 060 left them out of `save_disposition`, which reads lighting only, so Haste's `advantage` on Dexterity saves does nothing yet. Add the saver's timed `advantage` and `disadvantage` save modifiers here (matched like `save_modifier`: `ability: null` or the save's ability), so the "timed Advantage modifier" test below has a source. It moves batch numbers for fights where a Hasted creature makes a Dexterity save, which this ticket's version bump already covers.
- If 068 hasn't landed when this is built, add the "counts as Unconscious" helper here in the form 068 describes, and 068 reuses it.
- A downed PC caught in an area save therefore fails Strength and Dexterity saves. Damage to it still gives death save failures as `apply_damage` does now.
- Constitution, Wisdom, Intelligence and Charisma saves are unchanged.

Docs:

- `.agents/skills/homebrew-bench/references/live-mode.md`: one line where saves are described saying the engine fails Strength and Dexterity saves for these conditions and downed PCs, and gives a Restrained target Disadvantage on Dexterity saves.
- `CHANGELOG.md` under Changed: Strength and Dexterity saves now fail automatically for Paralyzed, Petrified, Stunned and Unconscious creatures and downed PCs, Restrained creatures have Disadvantage on Dexterity saves, batch numbers change, and the engine version moves.

Tests (`tdd`):

- Each of Paralyzed, Petrified, Stunned and Unconscious fails a Dexterity save and a Strength save without drawing a die: the stream position after the save equals the position before it, and the event names the condition and ability.
- A Paralyzed target makes a Constitution save normally, drawing one d20.
- A downed PC (`end_state` Unconscious, no conditions) and a Stable PC fail a Dexterity save automatically.
- A Restrained target rolls a Dexterity save with Disadvantage (two d20s, the lower kept) and a Strength save straight. With a sunlight-lighting Advantage source or a timed Advantage modifier as well, the Dexterity save is straight.
- Ask mode: an auto-failed PC save prompts nothing.
- A failed-save effect (damage on failure, a condition under `on_failure`) applies after an auto-fail exactly as after a rolled failure.

Versions and baseline:

- Bump the workspace version, and with it `ENGINE_VERSION`, by one patch from what `master` holds when this lands. Batch numbers change, and auto-fails draw fewer dice, so the stream moves. Don't regenerate `data/baseline/curve.json`. The engine stale reason is already accepted by `stale_beyond_the_engine` in `bench-core/tests/calibration.rs` and by the CLI test `batch_compares_against_the_embedded_curve_outside_the_repo`.
- `a_fixture_regenerates_to_its_committed_row` in `bench-core/tests/calibration.rs` regenerates goblin-warrior and compares it with the committed curve row. If the row differs and the test is not ignored yet, add `#[ignore = "curve stale until ticket 056 regenerates it"]`. Don't edit the committed row. Ticket 056 removes the ignore.
- Ticket 056 waits on this ticket so its one regeneration covers the change.
- Tickets 059, 062, 065, 066 and 068 also edit `engine.rs` and `CHANGELOG.md`, and several bump the version. Whichever lands second rebases.

Consult `tdd`, `codebase-design` and `code-review`.

## Resolution
