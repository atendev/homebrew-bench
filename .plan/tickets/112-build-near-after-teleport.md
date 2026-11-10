---
id: 112
title: Build Near after a teleport
type: task
mode: AFK
status: open
blocked_by: [108]
claimed: false
---

## Question

Build the decision in [Leftover Engaged range after a teleport](../closed/108-leftover-engaged-range-after-a-teleport.md): a teleport writes Near into the `ranges` entries of every pair it frees, behind a recorded setting so saved encounters refold as recorded.

## Notes

- Skills: `implement`, `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- `Effect::Teleport` in `bench-core/src/engine.rs` (~708) clears engagements first. Capture the pair set before that: the union of the subject's `engagement` list and every combatant whose list holds the subject. Write Near into both sides' `ranges` entries for each pair (`band` in `tactics.rs` looks up `a.ranges` then `b.ranges`). Other bands stay.
- Gate it on a new recorded setting, as `settings.reaction_windows` is (`encounter.rs` ~219, `live.rs` ~227, `batch.rs` ~674). New encounters record it on. A save without it replays as before, bands unchanged. State the legacy branch's removal plan in the ADR: it stays until a saves-migration ticket drops it.
- Amend ADR 0005's teleport sentence: gated, an engaged pair lands at Near and other bands stay, and Near is a modeling choice because the engine doesn't model where the subject lands. Legacy saves keep "bands unchanged". Say the fix covers existing entries only; a missing `ranges` entry still reads Engaged in `move_combatant` and Near in `band`, and stays out of scope (setup fills every cross-side pair, so it only arises for later additions).
- Edit ADR 0012's teleport paragraph: drop "leaves its range bands alone" and "may change this price". The price stays `threat(target) × SHORT_DISPLACEMENT_SHARE`.
- `live.rs` (~383) only prints starting bands in the START log, before any teleport, so live needs no change. Say so in the ticket resolution.
- Tests: with the setting off, the four self-teleport fixtures (blink dog, marilith, balor, nalfeshnee) give byte-identical results; with it on, a `target` teleport followed by `bench move` to Engaged needs 5 feet, not 0; a save without the setting replays unchanged; a one-sided engagement list still gets both entries written.
- Run the curve with the setting on and compare cells (the four self-teleport fixtures are in it). Regenerate and bump the policy version only if cells move, decided from that diff.

## Resolution

<Filled in at close.>
