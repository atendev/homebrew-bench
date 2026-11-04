---
id: 071
title: Build massive damage at 0 Hit Points
type: task
mode: AFK
status: open
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
