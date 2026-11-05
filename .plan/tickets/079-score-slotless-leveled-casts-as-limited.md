---
id: 079
title: Score slotless leveled casts as limited
type: task
mode: AFK
status: open
blocked_by: []
claimed: true
---

## Question

Graduated from Not yet specified, found by the 059 audit. `is_limited`, `slot_level` and `usage_word` in `bench-core/src/tactics.rs` read a cast's `slot_level` to tell a leveled cast from a cantrip. The engine reads the spell instead: `cast_level` takes a cast with no `slot_level` at the spell's own level, and `spend_slot` spends a slot of that level when the caster has `spell_slots`. Failure case: a homebrew PC with `spell_slots` whose Cure Wounds action is `{"cast": {"spell_id": "cure-wounds"}}` spends a level 1 slot on every cast, but the policy scores the cast as unlimited (no resource cost, and no limited-use bar in `pick_best`), and the plan's reason names no slot. Make the policy count a cast as using a slot whenever the engine would spend one.

Keep the current reading for a cast that names a `slot_level` on a creature with no `spell_slots` (14 dragon fixtures upcast this way): it stays limited, so SRD and standard-party batch numbers don't move. No fixture or party file has a slotless leveled cast on a creature with `spell_slots`. `POLICY_VERSION` stays 3. Bump the workspace version one patch. Since ticket 074, tests require a curve that matches the engine version, so regenerate `data/baseline/curve.json` and confirm that only `engine_version` changes.

## Notes

Consult `tdd` and `code-review`. The engine side is `cast_level` and `spend_slot` in `bench-core/src/engine.rs`.

## Resolution

