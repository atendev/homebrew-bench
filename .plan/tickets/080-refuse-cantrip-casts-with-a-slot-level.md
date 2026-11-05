---
id: 080
title: Refuse cantrip casts with a slot level
type: task
mode: AFK
status: open
blocked_by: []
claimed: true
---

## Question

Graduated from Not yet specified, found by ticket 070. `cast_level` in `bench-core/src/engine.rs` refuses any cast of a cantrip that names a `slot_level` ("is a cantrip and takes no spell slot"), so such an action is never ready and the batch policy skips it quietly. Load already refuses a cast whose slot is below the spell's level (`refuse_cast_below_spell_level` in `bench-core/src/stat_block.rs`). Failure case: a homebrew block whose action is `{"cast": {"spell_id": "fire-bolt", "slot_level": 1}}` loads clean and never fires in batch. Make load refuse it with the same check, so the author sees the fault.

A spell the effects file lacks stays with the missing-spell warning. If no fixture or party file has a cantrip cast with a slot level, SRD and standard-party numbers can't move, so `POLICY_VERSION` stays 3. Bump the workspace version one patch, regenerate `data/baseline/curve.json`, and confirm only `engine_version` changes.

## Notes

Consult `tdd` and `code-review`.

## Resolution
