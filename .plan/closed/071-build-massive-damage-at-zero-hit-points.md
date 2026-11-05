---
id: 071
title: Build massive damage at 0 Hit Points
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Kill a PC at 0 Hit Points when one hit's damage equals or exceeds its Hit Point maximum. The 068 audit found the gap: `apply_damage` in `bench-core/src/engine.rs` checks massive damage only on the hit that drops a PC from positive HP. Once the PC is dying (`EndState::Unconscious`) or Stable, any damage gives one death save failure (two on a critical) and never kills outright.

PHB ch. 1, "Damage and Healing", Damage at 0 Hit Points (p. 27): "If you take any damage while you have 0 Hit Points, you suffer a Death Saving Throw failure. If the damage is from a Critical Hit, you suffer two failures instead. If the damage equals or exceeds your Hit Point maximum, you die."

## Notes

- In the down branch of `apply_damage` (dying or Stable PCs, after 068), compare the damage taken after resistance and immunity, not the rolled total, with `max_hp`. If it is at or over `max_hp`, set `EndState::Dead` and log `<id> dies from massive damage`, as the drop-to-0 path does. Otherwise keep the current failure logic.
- Temporary HP soaks damage before this check, as it does elsewhere. Check what `apply_damage` already does and keep that order.
- Tests (`tdd`): a dying PC hit for exactly `max_hp` dies; one point under gives one failure; a Stable PC hit for `max_hp` dies; a critical at or over `max_hp` dies rather than taking two failures.
- Bump the workspace version, and with it `ENGINE_VERSION`, by one patch. Batch numbers may change. Don't regenerate `data/baseline/curve.json`; if `a_fixture_regenerates_to_its_committed_row` differs, add `#[ignore = "curve stale until ticket 056 regenerates it"]`. Ticket 056 waits on this so its one regeneration covers it.
- CHANGELOG under Changed. No live-state fields change.

Consult `tdd` and `code-review`.

## Resolution

The down branch of `apply_damage` now soaks the hit with Temporary Hit Points, then kills a dying or Stable PC whose remaining damage is at or over `max_hp`, logging `<id> dies from massive damage` with no failure counted. A lesser hit keeps the 068 logic: Stable becomes dying, one failure, two on a critical, death at three. `amount` arrives after `adjust_damage`, so resistance and immunity are already applied.

- Decision kept: a hit fully absorbed by Temporary Hit Points still gives a death save failure, as before. Only the massive-damage comparison uses the post-soak number.
- Tests. `damage_at_or_over_max_hp_kills_a_downed_pc` drives the goblin's 1d1 bite through `step`: dying at max HP 1 dies, max HP 2 takes one failure, Stable at max HP 1 dies, 1 Temporary HP soaks the hit to a failure, and an engaged critical (2 damage) on max HP 2 dies with no failures.
- Versions. Workspace and `ENGINE_VERSION` 0.1.10; no test pinned 0.1.9. No serialized field added. `a_fixture_regenerates_to_its_committed_row` still passes, so it isn't ignored; the curve wasn't regenerated. `cargo test --workspace` passes; clippy shows only master's two engine.rs errors. The SRD import exits 0 with 341 of 341 graduated.
- Audit fix, concentration. Before this, dropping to 0 HP, falling unconscious or dying never ended concentration, and an incapacitating condition didn't either; no other path handled it. Now `apply_damage` skips the concentration save when the hit drops an active creature and calls `break_concentration` after it falls unconscious or dies (monsters too; massive damage goes through `die_from_massive_damage`). The `Effect::Condition` arm calls `break_concentration` when the condition `incapacitates` (Incapacitated, Paralyzed, Petrified, Stunned, Unconscious, the same set `can_act` reads). Haste lethargy and Sleep take this path. A condition set by `bench override` doesn't: that stays the agent's call. Tests: `dropping_to_zero_hit_points_ends_concentration_without_a_save`, `an_incapacitating_condition_ends_concentration`, and a partial temp-HP soak case (temp 1, crit for 2, max 1: dies). Skipping the save draws one fewer d20, so batch numbers can shift; the calibration fixture row still matches.
- Refactor: `soak_temp_hp` serves both damage branches.
- Live-mode reference: no statement about damage to downed PCs or concentration, so no edit.
