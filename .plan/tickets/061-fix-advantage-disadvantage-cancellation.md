---
id: 061
title: Fix Advantage and Disadvantage cancellation on attack rolls
type: task
mode: AFK
status: open
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
- Ticket 056 waits on this ticket so its one regeneration covers the fix.
- Ticket 059 also edits `engine.rs` (casting) and `CHANGELOG.md`. Neither has to land first; the second rebases.

Consult `tdd`, `codebase-design` and `code-review`.

## Resolution
