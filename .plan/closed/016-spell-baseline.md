---
id: 016
title: Spell baseline
type: task
mode: AFK
status: closed
blocked_by: [015, 019, 020]
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

Built. `bench baseline generate --spells` runs every modeled SRD spell through the lift runner against the committed spell panel and writes `data/baseline/spell-lift.json`, embedded in the binary as the curve is. `bench baseline status --spells` (also `check`) compares the table's fingerprints against the inputs now and names each one that moved. Plain `status` and `check` still report the curve and panel only.

- **Table.** 94 spells and 355 rows: 206 measured, 81 `never_cast`, 68 `not_measured`. Per-member detail is left out, which keeps the file at 263 KB.
- **Fingerprints.** Party, policy, engine, fixture set, effects file, panel and fight count. `generate --spells` refuses a stale panel and a `--seed`, since the panel fixes the seeds.
- **Version.** Engine 0.1.47. The curve and panel differ from before only in `engine_version`.
- **Tests.** Calibration tests check that the table is current, covers every SRD spell and `plan_rows` row, and that Sleep regenerates to its committed rows. `cargo test` passes 641 tests.
- **Left for ticket 017.** The lift report does not yet warn when the table is stale, since nothing reads the table until then.

Council approved in round 1 (claude 8, gemini 10, gpt 9). Decisions are in [the decision report](../reports/016-decisions.md).

Timing: wall 45m, active 45m, user 0s, wait 3s, blocked 0s | implement 28m, audit 3m (1 round), implement-generate 9m, implement-tests 3m | tools 114 (3 subagent) | limits 0, compactions 0, handoffs 1 | ctx peak 156k | cost $3.69 sonnet-5-5 + $0.01 haiku-5-5
