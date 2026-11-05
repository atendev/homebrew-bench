---
id: 080
title: Refuse cantrip casts with a slot level
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Graduated from Not yet specified, found by ticket 070. `cast_level` in `bench-core/src/engine.rs` refuses any cast of a cantrip that names a `slot_level` ("is a cantrip and takes no spell slot"), so such an action is never ready and the batch policy skips it quietly. Load already refuses a cast whose slot is below the spell's level (`refuse_cast_below_spell_level` in `bench-core/src/stat_block.rs`). Failure case: a homebrew block whose action is `{"cast": {"spell_id": "fire-bolt", "slot_level": 1}}` loads clean and never fires in batch. Make load refuse it with the same check, so the author sees the fault.

A spell the effects file lacks stays with the missing-spell warning. If no fixture or party file has a cantrip cast with a slot level, SRD and standard-party numbers can't move, so `POLICY_VERSION` stays 3. Bump the workspace version one patch, regenerate `data/baseline/curve.json`, and confirm only `engine_version` changes.

## Notes

Consult `tdd` and `code-review`.

## Resolution

Built. `refuse_cast_below_spell_level` in `bench-core/src/stat_block.rs` now also refuses a cast of a cantrip that names any `slot_level`, 0 included, with `<ability-id>: a cantrip's cast names a slot level (<spell> is a cantrip and takes no spell slot)`. It matches the engine's `cast_level`, which refuses `Some(_)` on a level 0 spell. The check walks every cast in an entry through `casts_in`, so Multiattack steps and nested casts are covered, and a spell missing from the effects file still goes to the missing-spell warning. The batch-mode reference tells the agent to drop the cantrip's `slot_level` and revalidate.

Engine 0.1.17, `POLICY_VERSION` stays 3. The regenerated `data/baseline/curve.json` (1m40s) differs only in `engine_version`, and the auditor found no cantrip cast with a slot level among the 702 casts in fixtures and party files. Test: `a_cantrip_cast_with_a_slot_level_fails_at_load` (slot 0 and 1 refused, none accepted). Workspace tests pass. A haiku audit approved on the first round. No CI runs on branch pushes (`release.yml` runs on pull requests and tags only), so nothing was pushed.
