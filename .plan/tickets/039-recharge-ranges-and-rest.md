---
id: 039
title: Recharge ranges and recharge after rest
type: task
mode: AFK
status: open
blocked_by: []
claimed: false
---

## Question

Parse recharge dice 'N-6' or 'N' for N in 2..=6 in `roll_recharge`, reject unparseable dice at load time, relax the importer's recharge checks, and map recharge-after-rest to `PerRest { rest: short, uses: 1 }`.

## Notes

Consult `implement`, `tdd` and `domain-modeling`. Decision recorded in [closed/035](../closed/035-srd-importer-vocabulary-gaps.md). Unblocks monsters in the ticket 028 importer; rerun `python3 -I scripts/import-srd-monsters.py` and `cargo test -p bench-core` afterward and report the new graduated count.

## Resolution
