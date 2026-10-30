---
id: 028
title: SRD importer, review pipeline and fixtures
type: task
mode: AFK
status: open
blocked_by:
  - 025
  - 026
  - 036
  - 037
  - 038
  - 039
claimed: false
---

## Question

Build the SRD 5.2.1 monster importer and review pipeline, apply `data/srd/corrections.json` and the hand-authored effects layer, generate and commit `data/srd/import-review.md` (with its JSON twin), and graduate all 341 SRD monsters into `data/fixtures/`.

## Notes

Consult `implement`, `tdd`, and `domain-modeling`.

Key decisions to implement:
- **Conversion and effects mapping (`docs/adr/0005-closed-effect-vocabulary-with-agent-clauses.md`, `docs/adr/0008-import-review-with-correction-list.md`, closed tickets 008, 020, 022):**
  - Read `data/srd/2024/5e-SRD-Monsters.json` (pinned at `data/srd/COMMIT`), apply `data/srd/corrections.json` verbatim (failing outright if any correction fails to match upstream), and map abilities into the closed effect vocabulary using upstream structured fields plus the hand-authored effects layer where mechanics live in prose.
  - Fold the 34 `5e-database` swap-style `Multiattack` encodings (`(N - K)` unconditional attacks + `choose 1 of [K <X>, <Y>]`) into `{action: X, count: N}` with swap `<Y>`, and resolve spell display names in `action_options` against the monster's `Spellcasting` block (preserving `(level N version)` notes).
  - Classify every ability as `modeled`, `partly_modeled` (with `agent_clauses`), `inert`, or `unmodeled`, and attach one of the three default resolutions (`neutral`, `base_form`, `skipped`) to every unmodeled ability or agent clause.
- **Four-class review pipeline (`docs/adr/0008-import-review-with-correction-list.md`):**
  - Class (a): structured field contradicts the stat block's own prose (prose wins), or ability prose self-references a different creature name.
  - Class (b): structured field is missing that the prose implies (such as missing `damage` when `Hit:` specifies damage, or missing recharge on dual-breath dragons; zero-damage attacks like Roper `Tentacle` are not flagged).
  - Class (c): stat block contradicts an SRD 5.2.1 ruleset table (CR-to-XP or CR-to-PB, table wins).
  - Class (d): informational classification notes for every ability (`modeled`, `partly_modeled`, `inert`, `unmodeled` + default resolution), applied corrections, and spell ids missing from `data/spells/effects.json` (plus upstream spell id existence check).
- **Outputs and fixture graduation:**
  - Regenerate and commit `data/srd/import-review.md` and `data/srd/import-review.json`.
  - Exit non-zero if any blocking mismatch (classes a–c) remains open.
  - Graduate all 341 passing monsters to `data/fixtures/` and verify that every fixture validates against the `bench-core` stat block schema.

## Progress

Importer, review pipeline and fixture schema test are built (`scripts/import-srd-monsters.py`, `bench-core/tests/fixtures.rs`, `data/srd/import-review.{md,json}`, `data/fixtures/`). First pass graduated 186 of 341 monsters with 234 open blocking entries. The remaining blockers are vocabulary gaps decided in [closed/035](../closed/035-srd-importer-vocabulary-gaps.md) and built in tickets 036 to 039. After they land, rerun the importer, review the remaining entries (variable counts: hydra, medusa, barbed-devil; known audit gaps: self-reference detection beyond upstream names, spell id existence, `applies: false` path validation), and close this ticket when exit status is zero.

## Resolution
