---
id: 025
title: Workspace, schemas and CLI skeleton
type: task
mode: AFK
status: closed
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

Implemented in commit 3598236.

- Added Cargo workspace (`bench-core` library + `bench` binary) with dependencies for serde, schemars, clap, rand/rand_chacha, directories, toml, sha2, anyhow, thiserror, hex, and serde_path_to_error.
- Implemented canonical models in `bench-core/src/`:
  - `stat_block.rs`: 2024 stat block schema including identity, AC, HP, speeds (with per-mode `hover`), abilities, saves/skills, initiative, PB, damage/condition immunities, senses/languages, CR/XP/lair XP, traits/actions/bonus/reactions/legendary actions, PC fields, gear, pasted source, and intake flags. Four ability states (`modeled`, `partly_modeled`, `inert`, `unmodeled`) with optional default resolution.
  - `effect.rs`: closed effect vocabulary (attack, saving throw variants, damage, condition, timed modifier, roll modifier with lighting, forced move, heal, temp HP, teleport, move-then-attack, multiattack, use_action, cast, area with `origin`), reaction/passive triggers, and usage kinds.
  - `spell.rs`: spell effects file schema with scaling and delayed effects.
  - `encounter.rs`: encounter state, combatant state, engagement/range bands, action economy, command history, stat block snapshots, and tamper-hash helpers.
  - `rng.rs`: ChaCha8 wrapper with seed/cursor tracking, rewind, deterministic per-fight seed derivation using portable SHA-256 mixing, and pinned cross-platform test vector.
  - `config.rs` / `paths.rs`: `~/.bench/` layout, `config.toml`, home/workspace output directories.
  - `error.rs`: stable error codes (`invalid_json`, `schema_validation_failed`, `unmodeled_ability_refused`, `state_tampered`, `not_found`, `internal`) with distinct exit codes 1/2/3, plus `CodedError` with optional structured details.
  - `schema.rs`: JSON Schema generation via schemars and first-line validation via `serde_path_to_error` producing dotted field-path errors.
- Implemented thin CLI in `bench/src/main.rs`:
  - `bench help --json` prints the command catalog inside the JSON stdout envelope.
  - `bench schema monster` prints the stat block JSON Schema.
  - `bench schema validate <path>` validates a file and returns either an ok envelope or `schema_validation_failed` with dotted-path details.
  - All failures map to stable error codes/exit codes; JSON is emitted on stdout.
- Added unit tests covering schema round-trip, RNG determinism/replay, seed derivation, tamper hashing, snapshot verification, and CLI commands.
- Audited by a subagent in auto-mode; all blockers and all high issues except full JSON-Schema-only validation were resolved. Full schema-only validation is deferred to a later ticket and documented in `schema.rs`.

Status: closed.
