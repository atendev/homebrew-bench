---
id: 061
title: Fix Advantage and Disadvantage cancellation on attack rolls
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Make attack rolls follow the PHB rule: a roll has at most one Advantage and one Disadvantage, and if it has both they cancel, however many sources give each (PHB glossary "Advantage", p. 359). Today `disposition` in `bench-core/src/engine.rs` chains `improve` and `worsen`, so the result depends on the order the sources are read in.

## Notes

Found by the audit of [Encounter lighting setting](../closed/049-encounter-lighting-setting.md).

The fault: an Invisible, Poisoned attacker against a Paralyzed target reads Poisoned first (Disadvantage), then Paralyzed (straight), then Invisible (Advantage), and rolls with Advantage. By the rules it has both, so the roll is straight. `apply_modifier_target_disposition` chains the same way for timed modifiers.

Engine (`bench-core/src/engine.rs`):

- Collect two flags across every source: the actor's conditions, the target's conditions, Invisible on either side, and timed modifiers. Both set, or neither, is `Normal`; only Advantage is `Advantage`; only Disadvantage is `Disadvantage`.
- Put the rule in one helper that attacks use here and that saves use in ticket [060](060-build-encounter-lighting.md). Whichever of 060 and 061 lands second wires the other's sources into the helper. Remove `improve` and `worsen` once nothing calls them.
- Don't change which conditions count as sources. Whether the list matches the 2024 conditions (a Blinded target, a Restrained attacker) is a separate question. If the build finds gaps, record them in the Resolution and don't fix them here.
- The attack still draws two d20s every time, so the dice stream doesn't move; only which die is kept changes.

Tactics policy: `disposition` in `bench-core/src/tactics.rs` sums and clamps, and has the same fault. Leave it alone here. Ticket [056](056-build-v3-scoring-for-buffs-forced-movement-and-teleports.md) changes the policy to v3 and fixes it there, so the policy version moves once.

Tests (`tdd`):

- Two Advantages and one Disadvantage give a straight roll. So do one Advantage and two Disadvantages, and two of each.
- Two Advantages alone give Advantage, and two Disadvantages alone give Disadvantage.
- The result is the same for every order of the sources. Use a test that permutes them, or one that builds the same sources in two orders.
- The Invisible, Poisoned attacker against a Paralyzed target rolls straight.
- A timed modifier giving Advantage plus a condition giving Disadvantage give a straight roll.
- Existing single-source tests (`disposition_blinded_gives_disadvantage` and the rest) still pass.

Docs:

- `CHANGELOG.md` under Changed: attack rolls with both Advantage and Disadvantage from several sources are now straight, batch numbers can change for fights where that happens, and the engine version moves.

Versions and baseline:

- Bump the workspace version, and with it `ENGINE_VERSION`, by one patch from what `master` holds when this lands: 0.1.3 if ticket 059 has already moved it to 0.1.2, otherwise 0.1.2, in which case 059 takes 0.1.3. Batch numbers can change wherever a fight had mixed sources. Don't regenerate `data/baseline/curve.json`. The engine stale reason is already accepted by `stale_beyond_the_engine` in `bench-core/tests/calibration.rs` and by the CLI test `batch_compares_against_the_embedded_curve_outside_the_repo`.
- `a_fixture_regenerates_to_its_committed_row` in `bench-core/tests/calibration.rs` regenerates goblin-warrior and compares it with the committed curve row. If the row differs and the test is not ignored yet, add `#[ignore = "curve stale until ticket 056 regenerates it"]`. Don't edit the committed row. Ticket 056 removes the ignore.
- Ticket 056 waits on this ticket so its one regeneration covers the fix.
- Ticket 059 also edits `engine.rs` (casting) and `CHANGELOG.md`. Neither has to land first; the second rebases.

Consult `tdd`, `codebase-design` and `code-review`.

## Resolution

Built on branch `ticket-061-fix-advantage-disadvantage-cancellation`.

- `combined_disposition` in `bench-core/src/engine.rs` holds the PHB rule: it takes any number of dispositions, sets one Advantage flag and one Disadvantage flag, and returns `Advantage` or `Disadvantage` when only one is set, otherwise `Normal`. Ticket 060 is to feed its save sources (and the lighting sources for attacks) into this same function.
- `disposition` now collects its sources and passes them to the helper: the actor's conditions, the target's conditions, Invisible on either side, and the actor's timed modifiers through `modifier_dispositions`, which replaces `apply_modifier_target_disposition`. Its unused `default` parameter is gone. `improve` and `worsen` had no other callers and are removed.
- The condition list did not change. The attack still draws two d20s, so the dice stream does not move.
- New tests: two Advantages and one Disadvantage, one and two, and two of each roll straight; two of one kind keep that kind; every order of five source lists gives the same result; the Invisible, Poisoned attacker against a Paralyzed target rolls straight (it rolled with Advantage before); two timed Advantages plus Blinded roll straight. The old chain gave Advantage in both of the last two cases. `disposition_blinded_gives_disadvantage` still passes.
- The workspace is 0.1.2, and with it `ENGINE_VERSION`. `CHANGELOG.md` has the entry under Changed. `data/baseline/curve.json` is unchanged. `a_fixture_regenerates_to_its_committed_row` still passes, because the goblin-warrior row has no mixed-source attacks, so it is not ignored.
- `cargo test --workspace` passes. Clippy shows four warnings, all already on master; the fifth was the nested `if` in the removed function. The import review exits 0 with 341 of 341 graduated. The tactics policy's `disposition` is untouched for ticket 056.

Gaps in the condition list against the 2024 conditions (PHB Appendix C), not fixed here:

- Blinded target: attack rolls against it should have Advantage. Not a source.
- Restrained attacker: its attack rolls should have Disadvantage. Only the target side is a source.
- Petrified target: attack rolls against it should have Advantage. Not a source.
- Grappled attacker: Disadvantage on attacks against anyone but the grappler. Not a source.
- Prone target: Advantage only if the attacker is within 5 feet, otherwise Disadvantage. The engine always gives Advantage, ranged attacks included.
- Frightened attacker: Disadvantage only while the source of fear is in line of sight. The engine always applies it.
- Invisible on either side applies even when the other creature can see it (Blindsight, Truesight, See Invisibility).

These became ticket [067](../tickets/067-align-attack-roll-condition-sources-with-2024-rules.md).
