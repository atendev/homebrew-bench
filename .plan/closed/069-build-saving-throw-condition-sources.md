---
id: 069
title: Build saving-throw condition sources
type: task
mode: AFK
status: closed
blocked_by: [060, 065]
claimed: false
---

## Question

Make saving throws follow the 2024 conditions. Paralyzed (PHB p. 370), Petrified (p. 371), Stunned (p. 375) and Unconscious (p. 376) automatically fail Strength and Dexterity saving throws, and Restrained (p. 372) has Disadvantage on Dexterity saving throws. `resolve_save` in `bench-core/src/engine.rs` applies none of them.

## Notes

Surfaced by [Align attack-roll condition sources with the 2024 conditions](../closed/067-align-attack-roll-condition-sources-with-2024-rules.md), which left saves out. It waits on [Build encounter lighting](../closed/060-build-encounter-lighting.md), because 060 gives `resolve_save` a disposition through `combined_disposition`.

Engine (`bench-core/src/engine.rs`):

- Auto-fail: before any die is drawn, if the save's ability is Strength or Dexterity and the target has Paralyzed, Petrified, Stunned, or counts as Unconscious (ticket [068](../closed/068-build-attack-roll-condition-sources.md)'s helper: the Unconscious condition, or `end_state` Unconscious or Stable), the save fails. It draws no d20, and in ask mode it prompts no player. The event says why: `mia fails the save automatically (paralyzed, Dexterity)`, with `roll` and `total` left empty. The failure's effects (`on_failure`, full damage) apply as for any failed save.
- Restrained: a Dexterity save by a Restrained target adds Disadvantage as one more source in `save_disposition`, the list 060 passes to `combined_disposition`. Don't write a second helper. A Restrained target with an Advantage source from lighting or a modifier rolls straight. Strength saves are untouched.
- Timed modifiers: this ticket owns them on saves. 060 left them out of `save_disposition`, which reads lighting only, so Haste's `advantage` on Dexterity saves does nothing yet. Add the saver's timed `advantage` and `disadvantage` save modifiers as a source, keyed on the modifier's `bearer` that ticket [065](../closed/065-build-modifier-bearers-and-speed-modifiers.md) adds (through 065's `EncounterState::modifiers_on`, not `source`), and matched like `save_modifier`: `ability: null` or the save's ability. Test that Haste cast on an ally gives the ally Advantage on Dexterity saves and the caster none, so the "timed Advantage modifier" test below has a source. `save_disposition` serves every save (`resolve_save`, concentration and repeat saves), so these sources reach all of them. It moves batch numbers for fights where a Hasted creature makes a Dexterity save, which this ticket's version bump already covers.
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

Built on branch `ticket-069-build-saving-throw-condition-sources` by a delegated builder, test-first.

- Engine (`bench-core/src/engine.rs`). `automatic_save_failure(saver, ability)` returns the event message for a Strength or Dexterity save by a Paralyzed, Petrified, Stunned or counts-as-Unconscious saver (068's `counts_as_unconscious`). `resolve_save` checks it before any die: no d20, no ask-mode prompt, a `save` event `<id> fails the save automatically (<condition>, <Ability>)` with `roll` and `total` empty, then the usual failure effects and damage. `save_disposition` adds Restrained's Dexterity Disadvantage and the saver's timed `advantage`/`disadvantage` save modifiers, read through `modifiers_on` (the bearer) and matched by the new `saves_with(ability)`, which `save_modifier` now shares. `modifier_dispositions` takes a target predicate instead of one `ModifierTarget`. Haste's data already carried the Dexterity save Advantage, so `effects.json` is unchanged.
- `automatic_save_failure`, `save_disposition` and `save_modifier` are `pub(crate)` for ticket 056, whose notes now say the policy's `p_success` should call them.
- Tests: 9 new engine tests cover every bullet in Tests (each condition on Strength and Dexterity with the stream unmoved, a Paralyzed Constitution save, downed and Stable PCs, Restrained on Dexterity and Strength, sunlight and timed Advantage cancelling it, ask mode, failure effects after an auto-fail), Haste giving an ally Advantage on Dexterity saves and the caster none, and a Stunned creature's Strength repeat save.
- Versions. Workspace and `ENGINE_VERSION` 0.1.9; policy stays v3. No serialized field was added, so no old-state fixture test. `a_fixture_regenerates_to_its_committed_row` still passes, so it isn't ignored; the curve wasn't regenerated. `cargo test --workspace` passes; clippy shows only master's two engine.rs errors. The SRD import exits 0 with 341 of 341 graduated and `check-spell-effects.py` reports 84 entries consistent.

Readings chosen where the ticket was open:

- Repeat saves auto-fail too (`repeat_save` event, same message, no die), since the condition rule covers every Strength and Dexterity save. Concentration saves are Constitution and are unaffected.
- The auto-fail reason names Paralyzed, then Petrified, then Stunned, then Unconscious when several apply.
- Restrained's Disadvantage isn't named in the save event; only lighting traits are, as before.

Not done: the live-mode reference line. The shell guard held the edit to `.agents/skills/homebrew-bench/references/live-mode.md`, and the builder was told not to work around a hold, so it moved to ticket [072](../tickets/072-document-saving-throw-condition-sources-in-live-mode.md). Links to this ticket in 056 and closed 067 now point at `closed/`. No new fog.
