# Decisions: Spell baseline (ticket 016)

Sources: the ticket's Notes, ADR 0020, and choices made while building. The handoff from the first thread is folded in.

## Decisions made while building

1. **What `check` means.**
   - Question: the ticket asks for `bench baseline check --spells`, but no `check` command existed.
   - Options: add a separate `check` that exits non-zero when stale, or make `check` an alias of `status`.
   - Choice: alias of `status`. Both report `current` and the stale reasons, and both take `--spells`. Plain `status` and `check` still report the monster curve and the panel only.
   - Made by: implementer.

2. **Per-member detail.**
   - Question: keep each row's per-member results in the committed table?
   - Options: keep them, or drop them.
   - Choice: drop them (`members: []`). The table is 263 KB, and a homebrew run needs only the SRD rows' lift, interval and control win rate. A run that wants member detail reruns the spell.
   - Made by: implementer.

3. **`--seed` with `--spells`.**
   - Question: can the spell table take a seed?
   - Choice: no. The panel fixes every seed, so `--seed` with `--spells` is refused as `invalid_argument`. The flag became `Option<u64>` and the refusal is a JSON error, because a clap conflict prints plain text and the CLI's other refusals are JSON. A first test run failed on this and the change fixed it.
   - Made by: implementer.

4. **Fingerprints.**
   - Question: which inputs does staleness cover?
   - Choice: the curve's four (party, policy, engine, fixture set), plus the effects file hash, the panel fingerprint and the fight count. `generate` also refuses a stale panel, since every row would be stale at birth.
   - Made by: implementer, following the ticket's Notes.

5. **Stale-table warning in the lift report.**
   - Question: should a spell lift report warn when the table is stale?
   - Choice: not here. Nothing reads the table until ticket 017, so the warning belongs there.
   - Made by: implementer.

6. **Calibration tests.**
   - Choice: one test that the committed table is current, covers every SRD spell, and has exactly the rows `plan_rows` gives each spell with no member detail. One test that Sleep regenerates to its committed rows.
   - Made by: implementer.

7. **Version.** Engine 0.1.47. The curve and panel were regenerated at `--threads 4` and differ from before only in `engine_version`.
   - Made by: implementer.

## Result

94 spells and 355 rows: 206 measured, 81 `never_cast`, 68 `not_measured`. Generation took about 10 minutes at 4 threads. `cargo test` passes 641 tests. `bench baseline check --spells` reports the curve, panel and spell baseline all current.

## Audit

Council approved in round 1: claude 8, gemini 10, gpt 9. No blocking problems.

Declined minor problems, all from the claude seat alone:

- Nothing reads the table yet, so no report says when it is stale. Deferred to ticket 017 (decision 5).
- The CLI test checks `>= 94` spells and `>= 355` rows, not exact counts. Exact coverage is in the calibration test.
- The changelog's "curve and panel differ only in engine version" rests on the generated JSON, which the diff left out. The handoff recorded it and the regenerated files show a one-line change each.

## Surfaced

None.
