---
id: 060
title: Build encounter lighting
type: task
mode: AFK
status: open
blocked_by: [049]
claimed: false
---

## Question

Build what [Encounter lighting setting](../closed/049-encounter-lighting-setting.md) decided: one `lighting` value per encounter (`unspecified` by default, `dim_light`, `bright_light`, `sunlight`), set by `--lighting` on `bench start` and `bench batch` and changeable by override, with roll modifiers read from traits at roll time, a save disposition, and the 11 SRD lighting traits imported as roll modifiers.

## Notes

Model (`bench-core/src/effect.rs`, `encounter.rs`):

- Rename `Lighting::Normal` to `Lighting::Unspecified` (`unspecified`) with `#[serde(alias = "normal")]`, and keep it the default, so old homebrew and state files still load. Add `DimLight` (`dim_light`). Keep `BrightLight` and `Sunlight`.
- Add one method, such as `Lighting::meets(self, required: Lighting) -> bool`: `Sunlight` meets `Sunlight` and `BrightLight`, `BrightLight` meets `BrightLight`, `DimLight` meets `DimLight`, and `Unspecified` meets nothing.
- `EncounterSettings` gains `lighting: Lighting` with `#[serde(default)]`, so old state files load as `unspecified`.
- `schema_version` stays "1". Record it as an exception in ADR 0005, beside `no_provoke`: no committed stat block uses `normal`, and the alias keeps any homebrew file that does loading.

Engine (`bench-core/src/engine.rs`):

- Remove the `Effect::RollModifier` branch that pushes an `ActiveModifier`; it becomes unreachable.
- Stat-block validation (where stat blocks are checked at load, beside the recharge dice check) refuses a `roll_modifier` anywhere outside `traits`, including inside a multiattack, area or `on_hit`, with the reason "a roll modifier is a trait effect", and refuses one keyed to `unspecified`. `bench start`, `bench add` and `bench batch` all hit this check before any dice are drawn.
- Add one function that returns the lighting dispositions for a combatant and a roll kind: walk the combatant's stat block traits, keep each `roll_modifier` whose `lighting` the encounter's setting meets and whose `target` matches (`saving_throws` with `ability: None` matches every save, `Some(a)` only that ability), and return the dispositions with the trait name.
- `disposition` (attacks) takes them as one more source. Ticket [061](061-fix-advantage-disadvantage-cancellation.md) replaces the attack chain of `improve` and `worsen` with the same two-flag rule the save uses here; whichever of 060 and 061 lands second feeds lighting into that rule. Use one shared helper for the two-flag rule so attacks and saves can't drift.
- `resolve_save` gains a disposition worked out from two flags, any Advantage and any Disadvantage across every source: both set, or neither, is a straight roll; only one gives that disposition (PHB glossary "Advantage", p. 359). Don't chain `improve` and `worsen`. It draws a second d20 only when the result is Advantage or Disadvantage, so a straight save draws exactly what it draws today. In ask mode the second die prompts the player the same way the attack's second die does (`player_roll_in`).
- When a lighting modifier changed an attack or save, the event message names it: `(Disadvantage: Sunlight Sensitivity, sunlight)`. Other event text is unchanged.
- `ability_checks` modifiers are carried and do nothing; the engine rolls no checks.

Live (`bench/src/main.rs`, `bench-core/src/live.rs`, `digest.rs`, `log.rs`):

- `bench start --lighting <unspecified|dim_light|bright_light|sunlight>`, default `unspecified`, spelled in snake_case. It is stored in `settings`, so the initial snapshot and replay carry it.
- `bench override settings.lighting sunlight --reason <text>` must work, log, undo and replay. `settings` is already overridable and not hidden in the public view; add a test, and fix the path code only if the enum value doesn't apply.
- The digest and the public digest carry `lighting`. The log's setup lines name it.

Batch (`bench-core/src/batch.rs`, `baseline.rs`, `bench/src/main.rs`):

- `bench batch --lighting <value>`, default `unspecified`. `BatchSpec` gains `lighting`, and each fight's state takes it.
- The report's Standing assumptions table gains a fixed Lighting row: "Lighting: unspecified, lighting traits off" by default, otherwise "Lighting: dim light", "Lighting: bright light" or "Lighting: sunlight (also Bright Light)", each with "(set by `--lighting`)".
- The sample logs' STANDING ASSUMPTIONS block prints `lighting = <value>`. The JSON summary carries `lighting`. `Invocation` records the flag, and each sample's replay command adds `--lighting <value>` when it isn't the default.
- A modeled or partly modeled ability whose effects include a lighting roll modifier gets an ability-resolution line: "Applies in <light>; on in this run." or "...; off in this run." For a partly modeled trait that keeps a `skipped` default (the vampire Sunlight trait), `findings` in `batch.rs` (around lines 314 to 316) now checks lighting modifiers before the default resolution. When the modifiers ran, the lighting line replaces "Not modeled in phase 1; results may understate or overstate the monster." and names the agent clause left out, so a trait whose modifiers ran is not reported as unmodeled. The report's opening skipped-mechanics warning (around line 750) still names the trait for its clause.
- When the lighting isn't `unspecified`, the verdict's basis line adds "The baseline curve's fixtures fought with lighting unspecified."
- `run_fixture` in `baseline.rs` sets `unspecified` explicitly. The curve gains no lighting field.
- `POLICY_VERSION` stays at 2. The tactics policy doesn't read lighting.

Import (`scripts/import-srd-monsters.py`):

- Three exact sentence shapes in a trait become `roll_modifier` effects with `disposition: disadvantage`, each with a class (d) review note "lighting roll modifier from the prose":
  - "While in sunlight, the <creature> has Disadvantage on ability checks and attack rolls." and "While in sunlight, it has Disadvantage on attack rolls and ability checks.": `lighting: sunlight`, targets `attack_rolls` and `ability_checks` (ability null).
  - "While in sunlight, the <creature> has Disadvantage on D20 Tests.": `lighting: sunlight`, targets `attack_rolls`, `saving_throws` and `ability_checks` (ability null).
  - "While in Bright Light, the <creature> has Disadvantage on attack rolls.": `lighting: bright_light`, target `attack_rolls`.
- A trait that is only the sentence becomes `modeled` and drops its default resolution (kobold warrior, drider, specter, wight, wraith, shadow, cloaker). A trait with other sentences becomes `partly_modeled`: the vampire Sunlight trait (vampire spawn, vampire, vampire bat, vampire mist) keeps "The vampire takes 20 Radiant damage if it starts its turn in sunlight." as an agent clause and keeps its `skipped` default, so `batch::unresolved` and the curve's `ignored_abilities` don't change.
- Rerun the import and commit the 11 changed fixtures and the regenerated review list. The review must still exit 0 with all 341 graduated.

Tests (`tdd`):

- `Lighting::meets` for all sixteen pairs, and `normal` deserializing as `unspecified`.
- Engine: a kobold warrior attacks with Disadvantage under `sunlight` and straight under `unspecified`, `dim_light` and `bright_light`. The cloaker attacks with Disadvantage under `bright_light` and `sunlight`. The shadow saves with Disadvantage under `sunlight`, drawing two d20s and keeping the lower. A straight save draws one d20, so the stream position after it matches today's. Save dispositions with several sources: two Disadvantages and one Advantage give a straight roll, as do one Disadvantage and two Advantages, and the result doesn't depend on the order the sources are read in. A `roll_modifier` in an action, inside a multiattack or keyed to `unspecified` is refused at load, before `bench start` draws dice. The event names the trait.
- Ask mode: a save with Disadvantage prompts for two d20s.
- Live: `--lighting` sets the state and the digest; an override changes it, undo restores it, and replay re-derives the state hash; an old state file with no `lighting` loads as `unspecified`, and one holding `normal` loads too.
- Batch: the Lighting row, the log line, the JSON field, the replay string with and without the flag, the ability-resolution on/off lines, the vampire spawn's lighting line in place of the "Not modeled in phase 1" text under `sunlight`, and the basis-line note.
- Fixtures: the 11 fixtures carry the expected effects, and the vampire forms keep their clause and `skipped` default.
- No batch change under the default: for each of the 11 fixtures, a batch at `unspecified` with a fixed seed gives the same results as before this change. Check this from a scratch run against `master` before committing, and record it in the Resolution. If any number moves, stop and find out why before going on, because it would mean an `ENGINE_VERSION` bump this ticket doesn't plan for.

Docs:

- Amend ADR 0005: roll modifiers are trait effects read at roll time and gated on the encounter's lighting, the four values, the `unspecified` default and the sunlight-is-Bright-Light rule, saves can take a disposition, and the rename exception for `schema_version` with the `normal` alias.
- Amend ADR 0008: a third exact-sentence exception for the three lighting sentences, with the class (d) note.
- Amend ADR 0011: the curve is generated with lighting `unspecified`.
- `.agents/skills/homebrew-bench/references/live-mode.md`: replace "Lighting has no engine setting" with `--lighting` and the override, say what the engine does with it, and keep darkness, light sources and mixed lighting under agent judgment. Line 57's "Lighting" entry changes the same way.
- `.agents/skills/homebrew-bench/references/batch-mode.md`: `--lighting`, the default, and when to run a second batch (a homebrew monster with a sunlight or Bright Light trait). Tell the agent to encode a homebrew "While in sunlight / Bright Light" trait as a `roll_modifier` on the trait. Add guidance for Dim Light traits: "while in Dim Light" is a `roll_modifier` keyed to `dim_light`, which applies only under `--lighting dim_light`. "While in Dim Light or Darkness" is keyed to `dim_light` too, with an agent clause for the Darkness part, because Darkness is not a lighting value yet. Say that such a trait is off by default and that the user should run `--lighting dim_light` to see it. The skill test checks `--lighting` against `bench help --json`.
- `CHANGELOG.md` under Unreleased: the flag on both commands, the override, the digest field, the 11 fixtures now modeled with the vampire damage left as a clause, saves with Disadvantage from lighting, the `normal` to `unspecified` rename (old files still load), the new `dim_light` value, and that batch numbers under the default are unchanged.

Versions and baseline:

- No `ENGINE_VERSION` bump and no curve regeneration. The fixture set fingerprint changes; `stale_beyond_the_engine` in `bench-core/tests/calibration.rs` and the CLI test `batch_compares_against_the_embedded_curve_outside_the_repo` already accept that reason.
- Ticket [056](056-build-v3-scoring-for-buffs-forced-movement-and-teleports.md) waits on this ticket so its one regeneration covers the new fixtures.
- Ticket [061](061-fix-advantage-disadvantage-cancellation.md) also edits `disposition` in `engine.rs`. Either order works; see the engine section.
- Ticket [059](059-build-upcast-spell-scaling.md) touches `engine.rs` (casting), `bench/src/main.rs`, `live-mode.md`, `batch-mode.md`, ADR 0005 and `CHANGELOG.md`. The two can land in either order. Whichever lands second rebases onto the first, and if 059 has moved the version to 0.1.2, this entry stays under Unreleased.

Consult `tdd`, `codebase-design`, `domain-modeling` and `code-review`.

## Resolution
