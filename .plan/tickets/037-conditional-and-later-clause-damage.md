---
id: 037
title: Conditional and later-clause attack damage
type: task
mode: AFK
status: open
blocked_by: []
claimed: false
---

## Question

Add `Damage.if_target: Option<Condition>` with engine support, route later-clause unconditional damage to `Attack.on_hit`, update the importer for multi-damage and conditional damage, resolve the 'grappled by this creature' source-tracking question, review the 16 class-a Hit-clause entries, and verify the ADR 0005 wording matches what is built.

## Notes

Consult `implement`, `tdd` and `domain-modeling`. Decision recorded in [closed/035](../closed/035-srd-importer-vocabulary-gaps.md). Unblocks monsters in the ticket 028 importer; rerun `python3 -I scripts/import-srd-monsters.py` and `cargo test -p bench-core` afterward and report the new graduated count.

## Resolution
