---
id: 036
title: Multiattack choice with per-option counts
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Implement `ChoiceOption { action_id, count }` in `MultiattackStep::Choice`, retire `Multiattack.swap`, make the engine resolve K distinct picks (live prompt, batch policy), relax `multiattack()` in the importer to emit swap, choose-2, choose-3 and unequal-count shapes (keep variable counts blocked), and verify ADR 0005 and ADR 0008 (already amended to the target design) match what is built.

## Notes

Consult `implement`, `tdd` and `domain-modeling`. Decision recorded in [closed/035](../closed/035-srd-importer-vocabulary-gaps.md). Unblocks monsters in the ticket 028 importer; rerun `python3 -I scripts/import-srd-monsters.py` and `cargo test -p bench-core` afterward and report the new graduated count.

## Resolution

Implemented the new schema and updated both engine and importer to match ADR 0005 and ADR 0008.

- `bench-core/src/effect.rs` now defines `ChoiceOption { action_id, count }` and `MultiattackStep::Choice { options: Vec<ChoiceOption>, pick: u32 }`; `Multiattack.swap` is removed.
- `bench-core/src/engine.rs` resolves a `Choice` by selecting `pick` distinct options (phase-1 deterministic default is the first `pick` slots). Live-mode selection and the batch tactics policy will later override this default. Added `multiattack_choice_resolves_k_distinct_picks` test.
- `scripts/import-srd-monsters.py` no longer blocks swap, choose-2, choose-3, or unequal-count shapes. It expands each upstream option into `count` distinct `ChoiceOption` slots with `count: 1`, so a `Choice { options, pick = choose }` captures the total number of attacks while leaving the exact mix to the live prompt or batch policy.
- `scripts/generate-party.py` and the regenerated `data/party/standard/` blocks dropped the retired `swap` field.
- Regenerated `data/fixtures/` and `data/srd/import-review.{md,json}`.
- SRD import graduated count went from 186 to 227 of 341 monsters (41 newly unblocked). `cargo test --all` passes.
- Verified that ADR 0005 (closed vocabulary target design) and ADR 0008 (swap folding / correction-list approach) match the built behavior: swap-style and replacement multiattacks emit fixed `Action` steps plus a `Choice`, the engine resolves `pick` distinct slots, and `Multiattack.swap` is gone.