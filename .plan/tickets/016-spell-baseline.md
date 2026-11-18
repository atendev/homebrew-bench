---
id: 016
title: Spell baseline
type: task
mode: AFK
status: open
blocked_by: [015, 019, 020]
claimed: true
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

## Handoff

Branch `t3/spell-baseline`, wip commit on top of 4573047 (master `wayfinder/homebrew-bench-master` is also 4573047). Audit round: 0 (no council yet). Tag `wfm-homebrew-bench-016-1794999485`.

Done:
- `bench-core/src/spell_baseline.rs`: `SpellBaseline`, `SpellFingerprints` (curve's four + effects hash, panel fingerprint, fights), `generate` (runs `spell_lift::run` per SRD spell via `run_parallel`, members dropped, refuses a stale panel), `stale_reasons`, `embedded()`. 4 unit tests pass (`cargo test -p bench-core --lib spell_baseline`).
- `spell_lift` types gained `Deserialize`; `SpellBook::srd_entries`; `embedded::spell_lift_json` (file `data/baseline/spell-lift.json`).
- CLI: `bench baseline generate --spells` (out defaults to `data/baseline/spell-lift.json`, `--seed` conflicts), `baseline status|check [--spells]`. Two CLI tests added in `bench/tests/cli.rs`.
- Engine bumped to 0.1.47 (Cargo.toml, Cargo.lock). Curve and panel regenerated at `--threads 4`; both differ from before only in `engine_version`.
- ADR 0020 gained a "Spell baseline, as built" bullet.

Left, in order:
1. Release build is current. Run in background with a long timeout (the first try used 600000 ms and was killed): `nice -n 19 ./target/release/bench baseline generate --spells --threads 4` (355 rows, expect 15 to 25 min; the curve took 5.5 min). Do not use `pgrep -f`, it matches its own shell; use `pgrep -x bench`.
2. Rebuild (`nice -n 19 cargo build --release -p bench`), then `nice -n 19 cargo test` (cli test `baseline_check_with_spells_compares_the_embedded_spell_table` needs the table embedded; add a calibration test in `bench-core/tests/calibration.rs` that the committed table is current, covers every SRD spell and every `plan_rows` row, and one spell regenerates to its committed row).
3. CHANGELOG entry (engine 0.1.47, mention curve and panel differ only in version, rows count, not-measured and never-cast counts). Commit style: short, no co-author.
4. Council audit (rounds 1 to 5), decision report `.plan/reports/016-decisions.md` (unslop), Timing close, resolution into this ticket (delete this Handoff section), commit, report DONE. Do not touch map.md; return the Decisions line.

Open decisions to put in the report: `check` is an alias of `status` (no `check` command existed); table drops per-member detail; no seed flag with `--spells` (panel seeds); 017 reads the table, so the lift report does not yet warn on a stale table.

Read first: `bench-core/src/spell_baseline.rs`, `bench/src/main.rs` (`run_baseline`, `spell_baseline_status`).
