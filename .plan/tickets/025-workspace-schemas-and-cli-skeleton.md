---
id: 025
title: Workspace, schemas and CLI skeleton
type: task
mode: AFK
status: open
blocked_by: []
claimed: false
---

## Question

Set up the Rust Cargo workspace (`bench-core` library crate plus thin `bench` binary crate) and implement the canonical data models, JSON Schemas, ChaCha8 RNG wrapper, home/workspace path layout, and CLI skeleton (`bench schema` and `bench help --json`).

## Notes

Consult `implement`, `tdd`, `codebase-design`, and `domain-modeling`. Vocabulary lives in `CONTEXT.md`.

Key decisions to implement:
- **Workspace and binary (`docs/adr/0001-rust-engine-shipped-as-binaries.md`, `docs/adr/0002-one-shot-cli-with-state-files.md`, closed ticket 004):**
  - Cargo workspace with a core library crate (`bench-core`) and a thin CLI binary (`bench`).
  - `~/.bench/` (`%USERPROFILE%\.bench` on Windows) holds `config.toml` (`output = "home" | "workspace"`, `keep_encounters = true`, `delete_logs = false`, default batch `round_cap = 20`) and `index`.
  - JSON stdout envelope with `schema_version` and stable error codes; distinct exit codes for ok, rules refusal, and bad input.
  - `bench help --json` prints the machine-readable command catalog.
- **Stat block and effect vocabulary schemas (`docs/adr/0003-step-engine-with-abstract-engagement.md`, `docs/adr/0005-closed-effect-vocabulary-with-agent-clauses.md`, closed tickets 005, 013, 014):**
  - 2024 stat block layout: identity (`id`, `name`, `source`, `schema_version`), size/type/alignment, AC, HP (average + dice expression), speeds by mode + hover, ability scores, saving throw modifiers, skills, initiative modifier, proficiency bonus, damage vulnerabilities/resistances/immunities, condition immunities, senses + passive Perception, languages, CR/XP/lair XP, traits/actions/bonus actions/reactions/legendary actions (with legendary uses per round), optional PC fields (`class`, `level`, `spell_slots`, `dying` / `death_saves`), `gear`, pasted source text, and intake `flags` (field path + note).
  - Four ability states: `modeled` (`effects`), `partly_modeled` (`effects` + `agent_clauses`), `inert` (text kept, no combat effect), `unmodeled` (text only, plus optional default `resolution` in `neutral` / `base_form` / `skipped`).
  - Closed effect vocabulary types: attack roll, saving throw (half/no damage on success, repeat save, two-stage save, immunity after success, HP or creature-type gates), damage, condition (all 15 2024 conditions), timed modifier, roll modifier (with encounter `lighting` setting: `normal`, `sunlight`, `bright_light`), forced move, heal, temporary HP, teleport, move-then-attack, multiattack (ordered steps of `{action, count}` or `{choice}`, optional `swap`), `use_action`, `cast`, area (`cone`, `line`, `emanation`, `sphere`, `cube`, `cylinder` with size and origin), five usage kinds (`at_will`, `recharge` with optional `group`, `per_day`, `per_rest`, `cooldown_until_next_turn`), six reaction triggers, and six passive triggers.
  - Spell effects file schema (`id`, `name`, `source`, `effects`, concentration flag, per-effect `scaling`, delayed effects, optional `notes`) per `docs/adr/0014-spell-effects-file-authorship.md`.
  - `bench schema monster` (and schema validation helper) prints the JSON Schema and validates files with dotted field-path errors.
- **Encounter state model and ChaCha8 RNG (`docs/adr/0003-step-engine-with-abstract-engagement.md`, `docs/adr/0007-chacha8-stream-seeds-and-replay.md`, closed tickets 005, 012):**
  - Combatant state (HP, temp HP, max HP, conditions with source/duration, concentration, resources, action economy, engagement set and `engaged`/`near`/`far` range bands, end state: `active`, `unconscious`, `stable`, `dead`).
  - Encounter state (ID, engine version, origin directory, stat block snapshots + hashes, sides, combatants, round, initiative order, active turn, active effects, standing assumptions, command history, `rng: { algorithm: "chacha8", initial_seed, cursor }`, and tamper hash chain).
  - Deterministic ChaCha8 stream via `rand_chacha` with cursor tracking/rewind and deterministic per-fight seed derivation from `(master_seed, fight_index)`.

## Resolution
