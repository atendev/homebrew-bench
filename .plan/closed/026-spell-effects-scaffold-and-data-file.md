---
id: 026
title: Spell effects scaffold and data file
type: task
mode: AFK
status: open
blocked_by:
  - 025
claimed: false
---

## Question

Build `scripts/scaffold-spells.py`, author `data/spells/effects.json` covering the standard party spell list and core SRD monster combat spells in the closed effect vocabulary, update `data/party/standard-spells.json`, and add the SRD text snapshot honesty check.

## Notes

Consult `implement`, `tdd`, and `domain-modeling`.

Key decisions to implement:
- **Authorship and schema (`docs/adr/0005-closed-effect-vocabulary-with-agent-clauses.md`, `docs/adr/0014-spell-effects-file-authorship.md`, closed tickets 013, 023, 024):**
  - `scripts/scaffold-spells.py` reads `data/srd/2024/5e-SRD-Spells.json` and emits starter JSON entries for requested SRD spell `index` ids (mapping range, components, duration, damage dice, slot scaling, saving throws, and concentration).
  - `data/spells/effects.json` is keyed by SRD spell `index` strings. Each entry has `id`, `name`, `source` (`"SRD 5.2.1"`), `effects` (using the closed vocabulary from ticket 025), concentration flag, per-effect `scaling` rules, multi-part/delayed effects where needed, `agent_clauses` for riders outside the vocabulary, and optional `notes`.
- **Coverage scope:**
  - Every spell selected in `data/party/standard-spells.json` for the caster and healer archetypes across tiers 1–9 plus at-will options. Update `effects_status` in `data/party/standard-spells.json` from `"needs-effects-entry"` to reflect the completed entries.
  - Core combat damage, save, and condition spells referenced by SRD monster spellcasting blocks (per `docs/research/srd-ability-survey.md` and ticket 013) so spellcasting fixtures do not degrade unnecessarily to unmodeled defaults.
- **Honesty checks (`docs/adr/0014-spell-effects-file-authorship.md`):**
  - Validate `data/spells/effects.json` against the Rust spell-effects schema in `bench-core`.
  - Add a test/check that renders each modeled spell's expected mechanical behavior as text and diffs/verifies it against upstream SRD `description` and `higher_level` fields in `data/srd/2024/5e-SRD-Spells.json`.

## Resolution

- Created `scripts/scaffold-spells.py`, a deterministic helper that reads `data/srd/2024/5e-SRD-Spells.json` and emits starter `SpellEffect` JSON entries for requested SRD spell ids. It detects range, duration, concentration, attack/save, damage dice, slot scaling, area shapes, conditions, healing, teleports, and modifiers.
- Created `data/spells/effects.json` with 84 hand-finished spell entries: every caster and healer spell from `data/party/standard-spells.json` (tiers 1–9 plus at-will options), plus 25 core SRD monster combat spells. Each entry uses the closed effect vocabulary, includes `agent_clauses` for riders outside the vocabulary, and carries `source: SRD 5.2.1`.
- Updated `data/party/standard-spells.json` so every covered spell is marked `modeled` or `agent-only`, and the archetype `effects_status` fields now read `partially-modeled`.
- Added `SavingThrow::immunity_after_success` default in `bench-core/src/effect.rs` so omitting the field deserializes cleanly.
- Added a Rust schema-validation test in `bench-core/src/spell.rs` that fails the test suite if any entry in `data/spells/effects.json` does not deserialize into `SpellEffect`.
- Created `scripts/check-spell-effects.py`, the SRD-text honesty check. It renders each modeled spell's mechanical facts (damage dice, save abilities, conditions, area shapes, healing, upcast dice) and verifies that those concrete facts appear in the upstream SRD `description` and `higher_level` text. All 84 entries now pass.
- `cargo test` passes with 15 tests.
