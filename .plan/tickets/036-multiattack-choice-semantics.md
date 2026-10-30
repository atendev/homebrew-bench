---
id: 036
title: Multiattack choice with per-option counts
type: task
mode: AFK
status: open
blocked_by: []
claimed: false
---

## Question

Implement `ChoiceOption { action_id, count }` in `MultiattackStep::Choice`, retire `Multiattack.swap`, make the engine resolve K distinct picks (live prompt, batch policy), relax `multiattack()` in the importer to emit swap, choose-2, choose-3 and unequal-count shapes (keep variable counts blocked), and verify ADR 0005 and ADR 0008 (already amended to the target design) match what is built.

## Notes

Consult `implement`, `tdd` and `domain-modeling`. Decision recorded in [closed/035](../closed/035-srd-importer-vocabulary-gaps.md). Unblocks monsters in the ticket 028 importer; rerun `python3 -I scripts/import-srd-monsters.py` and `cargo test -p bench-core` afterward and report the new graduated count.

## Resolution
