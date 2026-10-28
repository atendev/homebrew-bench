# Spell effects file is scaffolded from SRD data and hand-finished

The engine models spells through an engine-owned effects file, keyed by SRD spell id (ADR 0005). For phase 1 the file must cover the standard party's minimum spell list and then grow on demand as fixtures graduate. We need a repeatable, checkable way to produce entries that stays aligned with upstream SRD text.

## Considered options

- **Hand-write every entry from scratch.** Rejected: it wastes the structured data in `5e-SRD-Spells.json` and is more error-prone than starting from a scaffold.
- **Convert directly from `5e-SRD-Spells.json`.** Rejected: the SRD fields do not speak the engine's effect vocabulary, and many mechanics live only in prose.
- **LLM-authored entries with human review.** Rejected: too much churn for a closed vocabulary; a deterministic scaffold plus hand-finish is easier to diff and review.
- **Scaffold from SRD fields, then hand-finish.** Adopted. The SRD gives range, components, duration, damage dice, slot scaling, and concentration; authors map these to the engine's effects and add `agent_clauses` for anything the engine cannot run.

## Consequences

- The file lives at `data/spells/effects.json`, keyed by SRD spell `index` strings, and is embedded in the `bench` binary at compile time.
- A standalone `scripts/scaffold-spells.py` emits starter JSON for requested ids.
- Each entry carries `id`, `name`, `source`, `effects`, and optional `notes`.
- Upcast is expressed as a per-effect `scaling` rule; multi-part spells are one entry with multiple timed effects; concentration is a boolean flag.
- Honesty checks are a snapshot diff against SRD text, regression batch fights against spellcasting fixtures, and import-review missing-id warnings.
- The concrete standard party spell list is resolved separately so the generator can reference it.
