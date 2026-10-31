---
id: 028
title: SRD importer, review pipeline and fixtures
type: task
mode: AFK
status: closed
blocked_by:
  - 025
  - 026
  - 036
  - 038
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

All 341 SRD 5.2.1 monsters graduate to `data/fixtures/`, and `scripts/import-srd-monsters.py` exits 0 with no open class a to c entries. The last 36 blocked monsters cleared like this:

- Multiattack steps can cast a spell through the Spellcasting action (`cast` on `MultiattackStep::Action` and `ChoiceOption`), which covers 18 dragons and the Cloud Giant. `MultiattackStep::Alternatives` runs one set of steps, which covers Medusa and Barbed Devil, and an empty set makes a step optional (Erinyes, Dryad). The engine takes the first set until live prompts and the batch tactics policy choose. A cast slot must name an action the creature has.
- A multiattack may name an action without its trailing parenthetical. Cast spells are checked against the SRD spell list (blocking) and `data/spells/effects.json` (a note).
- `corrections.json` went from 77 to 69 entries. The 10 werebeast entries were dropped because the converter already reads those forms correctly. Dryad and Erinyes were rewritten in the current shape. Hydra gets five Bites plus an agent clause. Archmage Misty Step gets a `desc` correction for "The mage", which the 2024 Monster Manual also prints.
- Half-Dragon's chosen-type Claw damage is an agent clause. The Rakshasa and Half-Dragon damage qualifiers are `intake_flags`.
- The self-reference check now reads every sentence-initial "The X" subject, case-insensitively, and checks it against other SRD creature names first. Otherwise a subject passes only if it is a word of the creature's own name, a word of the ability's name, a surveyed non-creature word, a capitalized rules term (condition or area shape), or a short name checked against the printed book (warhorse as "the horse"). Other SRD names compare word by word.

Known limits:

- The Hydra makes five Bites until the agent overrides the count, so it deals too much damage once a head dies.
- The subject check reads only sentence openings (after a period, colon, semicolon, "then" or "and"), so "If the mage fails" is never checked. A single-word creature name inside an ability name (Spider Climb) also lets a bare "the spider" pass. A capitalized rules term that is also a creature word ("The Cube" in another creature's prose) passes, because the check cannot tell the term from the Gelatinous Cube.
- `resolve_cast` ignores `slot_level` and `upcast`.

ADR 0005 and ADR 0008 are amended, and so is closed ticket 035.
