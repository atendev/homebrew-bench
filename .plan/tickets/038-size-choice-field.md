---
id: 038
title: Size choice field on stat blocks
type: task
mode: AFK
status: open
blocked_by: []
claimed: true
---

## Question

Add `size_choice: Vec<Size>` (default empty) to `StatBlock`, have the importer store the first listed size as `size` and the full list in `size_choice` for 'Medium or small' monsters, and regenerate the schema.

## Notes

Consult `implement`, `tdd` and `domain-modeling`. Decision recorded in [closed/035](../closed/035-srd-importer-vocabulary-gaps.md). Unblocks monsters in the ticket 028 importer; rerun `python3 -I scripts/import-srd-monsters.py` and `cargo test -p bench-core` afterward and report the new graduated count.

## Resolution
