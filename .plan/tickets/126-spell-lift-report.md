---
id: 126
title: Spell lift report
type: task
mode: AFK
status: open
blocked_by: [120, 125]
claimed: false
---

## Question

Build the batch report for a tested spell from [Spell value metric](../closed/001-spell-value-metric.md) ([ADR 0020](../../docs/adr/0020-spell-lift-as-the-spell-value-metric.md)): placement among same-level SRD spells, row by row.

## Notes

- Skills: `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- Per row: the spell's percentile among same-level SRD spells by mean lift, its nearest neighbours by name, and the single-monster and group columns. Neighbours whose intervals overlap read "level with". No pass or fail.
- A row with fewer than three modeled SRD spells is thin data and gets no percentile.
- "Reads like level N" appears only when the spell falls outside its own level's SRD range.
- Damage or healing per cast is shown ungraded.
- The report says batch measures the spell as a party tool only, and that monster testing and live mode cover the other side.
- Stale spell baseline flagged. Homebrew entry fingerprints and clause resolutions shown, as from [Batch homebrew clause resolution](120-batch-homebrew-clause-resolution.md).
- Both the JSON envelope and the text report.

## Resolution

<!-- Filled in at close. -->
