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
- Each entry carries `id`, `name`, `source`, `level`, `effects`, and optional `notes`. `level` is the spell's own level, 0 for a cantrip, and is checked against the SRD.
- Upcast is expressed as per-effect `scaling` rules; multi-part spells are one entry with multiple timed effects; concentration is a boolean flag.
- Amended by ticket 059 ([Upcast spell scaling](../../.plan/closed/050-upcast-spell-scaling.md)). A cast resolves at its cast level: `slot_level`, or the spell's `level` when the cast names none. A slot level below the spell's level, or any slot level on a cantrip, is refused. Each rule `{effect_index, per_slot}` applies once per level above `level`, and `per_slot` is one of four variants. `extra_dice {dice, damage_type?}` adds dice of the same size to the damage entry of that type (else the first), or to Heal or TempHp dice, looking inside an Area. `extra_modifier {amount}` adds a flat amount to Heal, TempHp or Damage. `extra_targets {count}` raises `target_count` on an Attack or Area and changes nothing on other effects, which already take every target passed. `extra_effects {count}` appends copies of the effect, for darts and rays. `increase_duration` and `add_damage_type` are retired. An optional `upcast_clause` holds what a higher slot does that the engine does not apply: an upcast of such an entry logs an `upcast_clause` event, and a batch warns once per such cast in the monster's stat block. A cantrip never takes a rule; its damage grows with the caster's level at 5, 11 and 17. The scaffold reads the higher-slot text from `higher_level` and the description's "Using a Higher-Level Spell Slot." paragraph, and `check-spell-effects.py` requires every spell with that text to have a rule or a clause and each rule's dice or amount to appear in it.
- Honesty checks are a snapshot diff against SRD text, regression batch fights against spellcasting fixtures, and import-review missing-id warnings.
- The concrete standard party spell list is resolved separately so the generator can reference it.
