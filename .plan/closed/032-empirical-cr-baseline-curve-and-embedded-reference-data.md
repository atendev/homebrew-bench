---
id: 032
title: Empirical CR baseline curve and embedded reference data
type: task
mode: AFK
status: closed
blocked_by:
  - 031
claimed: false
---

## Question

Generate and commit the empirical CR baseline curve in `data/baseline/` across all graduated SRD fixtures, wire baseline comparisons and effective CR into `bench batch`, embed all four reference datasets into `bench` at compile time, add `bench fixture` and `bench party` inspection commands, and add spellcasting fixture regression tests.

## Notes

Consult `implement`, `tdd`, and `domain-modeling`.

Key decisions to implement:
- **Empirical CR baseline curve (`docs/adr/0011-empirical-fixture-curve-as-cr-baseline.md`, closed tickets 015, 017):**
  - Run all graduated fixtures in `data/fixtures/` against the standard party across levels 1–20 (100 fights per cell) under the batch tactics policy and commit the resulting table and input fingerprints (fixture set hash, party table hash, tactics policy version/constants, engine version) under `data/baseline/`.
  - Compute calibrated difficulty bands per `(CR, party_level)` cell using project calibration thresholds (`Low`: >= 95% party wins and < 2% fights with a PC death/casualty; `Moderate`: >= 75% party wins and < 15% fights with a PC death; `High`: anything riskier, where a casualty is a PC death, not a downed-and-saved PC).
  - Pool `±1` CR when a CR has too few fixtures; label cells that remain thin as `thin data` rather than smoothing silently.
  - Wire baseline comparison into `bench batch` reports: four-part neutral mismatch statement (verdict at claimed CR against the fixture band in both directions including `"no threat at this level"`, whole-level `Effective CR` clamped to `"around 1 or trivial"` below 1 and `"over 20"` above 20, round/DPR anomalies against the fixture spread with thin-data cells named, and one-sentence basis line), stale-baseline fingerprint warning when inputs differ, SRD factual layer (XP and encounter budget difficulty), and the verbatim SRD 5.2.1 attribution footer.
- **Compile-time embedded data and inspection CLI (`docs/adr/0013-embedded-reference-data-and-skill-packaging.md`, closed ticket 019):**
  - Embed `data/fixtures/`, `data/spells/effects.json`, `data/party/`, and `data/baseline/` into `bench` at compile time (`include_str!` / `include_bytes!`).
  - Resolve bare fixture slugs in `bench start` and `bench batch` against embedded fixtures when the argument is not a path on disk, and default `bench batch` to the embedded standard party and baseline curve.
  - Implement read-only inspection commands `bench fixture list`, `bench fixture show <id>`, and `bench party show [--level <1-20>]` printing embedded JSON to stdout.
- **Spell honesty check (`docs/adr/0014-spell-effects-file-authorship.md`, closed ticket 023):**
  - Add regression batch fight tests against representative spellcasting fixtures asserting expected difficulty-band ranges and verifying that missing monster spells do not silently break calibration.

## Resolution

The curve, the batch comparison, the embedded data and the inspection commands are built.

- `data/baseline/curve.json` holds all 341 SRD fixtures at 100 fights per party level 1 to 20, a per-fixture row, 620 per-`(CR, level)` cells (60 thin) and the fingerprints (fixture set, party, tactics policy id, version and constants, engine version). Hashes go through `serde_json::Value` first so hash-map fields cannot change them. Each fixture derives its seed from the base seed `20261102` and its id, so the file regenerates byte for byte, and a test recomputes one fixture to prove it.
- 174 fixtures carry agent clauses or unmodeled abilities. They fight with those left out (`abilities.<id>=ignored`), listed per fixture under `ignored_abilities`, so the curve is the monster as the engine plays it. The alternative, dropping them, would have left half the fixtures out.
- A CR with fewer than three fixtures pools the CRs one ladder step either side; a cell still short of three is thin. CRs 26 to 28 have no fixtures and no cell.
- `bench batch` uses the embedded curve by default. Its summary and report carry the four-part statement (verdict at the claimed CR both ways with "no threat at this level", effective CR as "around 1 or trivial" or "over 20" at the ends, rounds and damage-per-round anomalies naming thin cells, one basis line), the SRD XP and four-character budget facts, and a stale-baseline warning naming each input that differs.
- `build.rs` embeds the four data roots. Bare fixture or standard party ids resolve in `start`, `add` and `batch` when no file of that name exists. `bench fixture list|show`, `bench party show [--level]`, `bench baseline status|generate` are new.
- Spell honesty: batch warns for every spell id a monster casts that the effects file lacks (only `cloud-giant`'s `fog-cloud` today), the curve records them per fixture, and `bench-core/tests/calibration.rs` pins band ranges for archmage, mage, priest, couatl and lich.
- Found and fixed: the tactics policy recursed forever when two combatants each had condition effects (the Dryad overflowed the stack). A control effect nested more than two threat evaluations deep now counts as zero. That changes scoring, so the policy is v2 and the curve was generated under it.
