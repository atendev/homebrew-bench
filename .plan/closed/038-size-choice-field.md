---
id: 038
title: Size choice field on stat blocks
type: task
mode: AFK
status: closed
blocked_by: []
claimed: true
---

## Question

Add `size_choice: Vec<Size>` (default empty) to `StatBlock`, have the importer store the first listed size as `size` and the full list in `size_choice` for 'Medium or small' monsters, and regenerate the schema.

## Notes

Consult `implement`, `tdd` and `domain-modeling`. Decision recorded in [closed/035](../closed/035-srd-importer-vocabulary-gaps.md). Unblocks monsters in the ticket 028 importer; rerun `python3 -I scripts/import-srd-monsters.py` and `cargo test -p bench-core` afterward and report the new graduated count.

## Resolution

Added `size_choice: Vec<Size>` to `StatBlock` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]` for backward compatibility. Updated the SRD importer to split upstream size strings on `" or "`, storing the first choice as `size` and the full list in `size_choice` when there are multiple choices. This handles the 30 SRD monsters with `"Medium or small"` size. Updated all three test `StatBlock` literal constructions in `bench-core`.

SRD fixtures graduated: **299 of 341** (up from 269).

Branch: `ticket-038-size-choice`, commit `9fb7f96`.
