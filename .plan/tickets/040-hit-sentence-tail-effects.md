---
id: 040
title: Hit-sentence tail effects
type: task
mode: AFK
status: open
blocked_by: []
claimed: false
---

## Question

The importer keeps only later sentences of an attack's Hit clause as agent clauses. Effects stated in the Hit sentence after the damage are dropped without a trace: Clay Golem's Hit Point maximum loss, Otyugh's Poisoned condition, and others. Capture the tail of the Hit sentence as an agent clause, the way the saving-throw path already keeps its remainder, and check which fixtures change.

## Notes

Consult `implement`, `tdd` and `domain-modeling`. Raised by the ticket 037 audit; see [closed/037](../closed/037-conditional-and-later-clause-damage.md). The tail may start with "and" or ", and" once the damage phrases are removed, so decide how to trim it. Rerun `python3 -I scripts/import-srd-monsters.py` and `cargo test --all` afterward and report which fixtures gained clauses.

## Resolution
