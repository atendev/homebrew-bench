---
id: 079
title: Score slotless leveled casts as limited
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Graduated from Not yet specified, found by the 059 audit. `is_limited`, `slot_level` and `usage_word` in `bench-core/src/tactics.rs` read a cast's `slot_level` to tell a leveled cast from a cantrip. The engine reads the spell instead: `cast_level` takes a cast with no `slot_level` at the spell's own level, and `spend_slot` spends a slot of that level when the caster has `spell_slots`. Failure case: a homebrew PC with `spell_slots` whose Cure Wounds action is `{"cast": {"spell_id": "cure-wounds"}}` spends a level 1 slot on every cast, but the policy scores the cast as unlimited (no resource cost, and no limited-use bar in `pick_best`), and the plan's reason names no slot. Make the policy count a cast as using a slot whenever the engine would spend one.

Keep the current reading for a cast that names a `slot_level` on a creature with no `spell_slots` (14 dragon fixtures upcast this way): it stays limited, so SRD and standard-party batch numbers don't move. No fixture or party file has a slotless leveled cast on a creature with `spell_slots`. `POLICY_VERSION` stays 3. Bump the workspace version one patch. Since ticket 074, tests require a curve that matches the engine version, so regenerate `data/baseline/curve.json` and confirm that only `engine_version` changes.

## Notes

Consult `tdd` and `code-review`. The engine side is `cast_level` and `spend_slot` in `bench-core/src/engine.rs`.

## Resolution

Built. A new `cast_slot(block, cast)` in `bench-core/src/tactics.rs` reads a cast's slot as the engine spends it: the named `slot_level`, or, when the cast names none and the caster has `spell_slots`, the spell's level from the effects catalog. A cantrip, a spell missing from the catalog, or a slotless cast on a creature without `spell_slots` gives none. `is_limited`, `slot_level` and `usage_word` now take the stat block and go through `cast_slot`, so the limited-use bar in `pick_best`, the resource cost in every plan, the reason's `level N slot` and the cheapest-reaction order in `decide_hit` all agree with the engine. `teleport_plan` and `modifier_plan` take the block to reach it.

A cast that names a slot level on a creature with no `spell_slots` (the 14 dragon upcasts) still counts as limited, though the engine spends nothing for it. That mismatch already existed and is left alone, because changing it would move SRD numbers.

Engine 0.1.16, `POLICY_VERSION` stays 3. Since ticket 074 the tests require a curve that matches the engine version, so `data/baseline/curve.json` was regenerated (1m53s). It differs from the committed curve only in `engine_version`, which confirms that no SRD fixture or standard-party cell moves. Test: `a_slotless_leveled_cast_uses_a_slot_only_when_the_caster_has_slots`. Workspace tests pass. A haiku audit approved the change on the first round. No CI runs on branch pushes (`release.yml` runs on pull requests and tags only), so nothing was pushed.
