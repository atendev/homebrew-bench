---
id: 023
title: Spell effects file authorship
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

How is the engine-owned spell effects file produced and checked? Monster stat blocks reference 116 distinct spells through structured spellcasting blocks and 23 more by name. How much of that set is hand-modeled first, in what order, and what check keeps the file honest against upstream spell text?

## Notes

ADR 0005 routes spells through an effects file keyed by SRD spell id; a spell id absent from the file makes the cast an unmodeled ability with a default resolution. The import review (closed ticket 020) records missing ids as classification notes and checks only that referenced ids exist upstream, so this ticket owns the file's authorship process. The survey (`docs/research/srd-ability-survey.md`) lists which monsters carry spellcasting blocks. The future spells map may replace the file, but phase 1 needs enough of it that spellcasting monsters do not degrade entirely to default resolutions. The Standard party build decision (closed ticket 014) sets the phase-1 minimum for the file: the caster archetype needs 3 to 5 leveled spells per tier covering single-target damage, area damage, save-based control, and one buff, plus one at-will damage option, and the healer needs 2 to 3 healing spells plus one at-will option, with no summons and no zones. Consult `grilling` and `domain-modeling`.

## Resolution

**Coverage target.** Phase 1 ships the standard party's minimum spell list first, then grows the file on demand as fixtures graduate. Monster spells beyond the party list are added only when import-review warnings show a passing fixture being degraded by a missing entry. This keeps the file honest and avoids guessing which monster spells matter before the baseline curve exists.

**Authorship process.** A standalone `scripts/scaffold-spells.py` reads `data/srd/2024/5e-SRD-Spells.json` and emits a starter effect JSON for one or more requested SRD spell ids. Authors hand-finish the entry in the engine's closed effect vocabulary, add `agent_clauses` and `notes` where needed, and cite the source as SRD 5.2.1.

**File location and loading.** The file lives at `data/spells/effects.json`, keyed by SRD spell `index` strings. It is embedded in the `bench` binary at compile time alongside fixtures and the standard party, matching the packaging decision (ADR 0013).

**Entry schema.** Each entry carries `id`, `name`, `source`, `effects`, and optional `notes`. Effects use the existing closed vocabulary. Upcast is a per-effect `scaling` rule (for example, "+1d6 per slot level above 3"). Multi-part spells are one entry with multiple effects, including delayed damage with a `delay` key. Concentration is a boolean flag; ongoing effects carry explicit durations rather than parsing SRD duration strings.

**Honesty checks.** Three overlapping checks keep the file aligned with upstream:
1. CI renders each modeled spell's expected behavior as text and diffs it against SRD `description`/`higher_level` snapshots.
2. Regression batch fights against spellcasting fixtures assert expected difficulty-band ranges.
3. Import review reports referenced spell ids that are missing from the effects file; for phase 1 this is a warning, not a blocker, because the fallback default resolution is an explicit design choice (ADR 0005).

**Follow-up.** The concrete standard party spell list is split out into ticket 024.
