---
id: 005
title: Monster schema and engine state model
type: grilling
mode: HITL
status: closed
blocked_by: [001, 002, 003]
claimed: false
---

## Question

What fields does a homebrew monster have, and what state does the engine track per creature and per fight? How does the engine advance a fight: whole-fight simulation, one action at a time, or both?

## Notes

The state model has to support both modes. Consider HP, conditions, concentration, resources, recharge, action economy, and whether state can be read and resumed between steps. Also open: whether the engine tracks position, and how position-dependent abilities (area target counts, Pack Tactics, auras) get resolved either way.

## Resolution

Decided with the user on 2026-10-24. The step function and position model are recorded in `docs/adr/0003-step-engine-with-abstract-engagement.md`. New terms (stat block, combatant, engaged, modeled ability) are in `CONTEXT.md`.

**Advancing a fight.** One step function sits at the core. Live mode calls it once per command, and `bench batch` loops it internally with a built-in tactics policy. What one step covers is for Live mode turn contract.

**Position.** No grid. Each combatant tracks its engagement set and a range band (engaged, near, far) to each opposing side. Engaging or leaving engagement costs 5 ft, and moving between near and far costs 30 ft. Leaving engagement without Disengage provokes Opportunity Attacks from everyone it was engaged with. Reach 10 ft counts as engaged for that attacker. Flying and out-of-reach creatures carry a flag: the agent handles them in live mode, the tactics policy in batch. Pack Tactics checks whether an ally is engaged with the target, and auras affect engaged combatants. In live mode the agent names area targets and the engine checks them against the bands. Batch uses a default target count by area size, capped by the opponents engaged with or near the source.

**Stat block format.** JSON, validated against a JSON Schema the binary prints (for example `bench schema monster`). Invalid files are rejected with field paths. Fields follow the 2024 layout:

- identity: `id`, `name`, `source` (srd or homebrew), `schema_version`
- size, type, alignment (text only)
- AC, HP (average plus dice expression), speeds by mode including hover
- ability scores, saving throw modifiers, skills, initiative modifier, proficiency bonus
- damage vulnerabilities, resistances and immunities, condition immunities
- senses with passive Perception, languages as text
- CR, XP, and XP in lair as data only
- traits, actions, bonus actions, reactions and legendary actions, with legendary uses per round
- optional PC fields: class and level as labels, spell slots, death saves
- `gear` as text, the source text the user pasted, and a `flags` list of intake problems (field path plus note)

Each ability has `name`, original `text`, `kind`, optional `usage` (recharge X-Y, N per day, N per rest), legendary `cost`, and either `effects` (modeled) or `unmodeled: true`. The effect vocabulary is still fog.

**Party members** use the same stat block schema with the optional PC fields.

**Combatant state.** HP, temp HP and max HP; conditions with source and duration; concentration and the effects tied to it; resources (recharge state, per-day and per-rest uses, legendary actions left this round, Legendary Resistance uses, spell slots); action, bonus action, reaction and movement left; engagement and range bands; end state (active, unconscious with death saves, stable, dead). Monsters die at 0 HP by default.

**Durations** anchor to the start or end of a named combatant's turn, with optional repeat saves and a round count for minute-long effects.

**Encounter state.** Encounter ID, engine version, origin directory, a snapshot of each stat block with its hash, sides, combatants, round, initiative order, whose turn it is, active effects, standing assumptions and overrides applied, and the last log hash. Editing a stat block mid-fight is an override with a reason. Randomness state is listed, and its shape belongs to the seeds-and-replay fog item. Reaction windows belong to Live mode turn contract.

**Initiative.** One roll per stat block group by default, with a per-encounter flag for individual rolls.

**Unmodeled abilities in batch.** `bench batch` refuses to run until every unmodeled ability has a resolution: a standing assumption (a typed approximation) or "ignored". The report lists both kinds.

**Ending an encounter.** It ends when every combatant on one side is dead, unconscious or stable. Batch adds a round cap (default 20 in `config.toml`, overridable per batch) and records a draw when it's reached. Fleeing and surrender are agent judgment in live mode only.

**SRD import.** The script builds stat blocks from upstream structured fields, checks them against the prose, and writes mismatches to a review list. Prose wins when the two disagree. Example: upstream lists Adult Red Dragon's Multiattack as two Rends plus a choice, while the prose says three. A monster becomes a fixture only after its review entries are fixed.

**Surfaced.** The batch default for area target counts goes to the tactics fog item. The SRD import review is a new fog item.
