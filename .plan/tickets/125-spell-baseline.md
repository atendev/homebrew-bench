---
id: 125
title: Spell baseline
type: task
mode: AFK
status: open
blocked_by: [122, 123, 124]
claimed: false
---

## Question

Build the committed spell baseline from [Spell value metric](../closed/001-spell-value-metric.md) ([ADR 0020](../../docs/adr/0020-spell-lift-as-the-spell-value-metric.md)): SRD lift for every modeled spell at every row, under `data/baseline/`, with fingerprints and staleness.

## Notes

- Skills: `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- `bench baseline generate --spells` and `bench baseline check --spells`, beside the monster curve (`bench/src/main.rs` ~342, `run_baseline`). Plain `bench baseline check` keeps reporting the monster curve only.
- Fingerprints: effects file, party, panel, policy, engine and fight count. A report computed against a stale table says so, as the monster curve does (`Baseline::stale_reasons`).
- Embed the table in the binary as the curve is.
- A homebrew run fights only its own spell and the control, and reads SRD rows from this table.
- Generate and commit the table. Changelog entry.

## Resolution

<!-- Filled in at close. -->
