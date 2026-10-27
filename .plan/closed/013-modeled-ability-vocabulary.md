---
id: 013
title: Modeled-ability vocabulary
type: grilling
mode: HITL
status: closed
blocked_by: [008]
claimed: false
---

## Question

Which mechanics does the engine model, and what is the effect vocabulary for the `effects` field on an ability? Where is the line between a modeled ability and an unmodeled one, and what does the agent do at the line?

## Notes

The stat block schema from "Monster schema and engine state model" gives each ability either `effects` (modeled) or `unmodeled: true`. The vocabulary was left open. Use the SRD ability survey findings for the real distribution of patterns. Consult `grilling` and `domain-modeling`. SRD import review graduates from the fog once this closes.

Starting positions from the SRD ability survey, agreed with the user on 2026-10-26. Confirm or change each one in the session.

- Spellcasting stays in the stat block as a structured block with spells referenced by id. The engine models a spell only when it fits the vocabulary (save for damage, condition, area). Any other spell is an unmodeled ability. No spell system in phase 1, and stable spell ids leave room for the future spells map.
- Reactions get a window only when their trigger is on a small closed trigger list (hit by an attack, targeted by an attack, takes damage, a creature leaves reach, a creature casts a spell, to be fixed from the real trigger phrases). Other reactions stay unmodeled and the agent handles them, as ADR 0004 says.
- Persistent zones, created creatures and objects with their own HP stay unmodeled in phase 1. Revisit if early reports show many monsters blocked by them.
- Refine the batch refusal rule from the state-model ticket: the SRD import ships a default resolution for each unmodeled pattern, labeled in the report (revival and senses are "no combat effect", Shape-Shift is "stays in its base form"). The user can override each one. Homebrew abilities still need an explicit resolution.
- The survey script's tag table is a cross-check, not the importer's parser.

Three counts to run at the start of the session, from the survey script: how often each spell appears in spellcasting blocks, the distinct reaction trigger phrases, and the three unmodeled entries missing from the survey's prose list (it lists 80 of 83). The findings and script are on branch `research/srd-ability-survey` until that branch is merged.

## Resolution

Decided with the user on 2026-10-26. Recorded in `docs/adr/0005-closed-effect-vocabulary-with-agent-clauses.md`. New terms (agent clause, modifier) are in `CONTEXT.md`. The survey findings were read from branch `research/srd-ability-survey`, which is not yet merged, so ticket 008 is still open on this branch.

- **Closed vocabulary.** The engine runs only a fixed list of effects and never reads prose. Effects: attack roll, saving throw (half or no damage on success, repeat save, two-stage save, immunity after success, gates on current HP or creature type), damage, condition, modifier, roll modifier, forced move, heal, temporary HP, teleport, move-then-attack, multiattack, use-action, cast, and an area (cone, line, emanation, sphere, cube, cylinder, each with size and origin). The user confirmed every item below in the session. The survey's line for attack, save, damage, condition, area, healing and the movement actions was adopted as written, without a question for each.
- **Four ability states.** Modeled (`effects`), partly modeled (`effects` plus `agent_clauses`), inert (no combat effect, text kept), unmodeled (text only). A partly modeled ability runs through the engine except for its agent clauses. This refines ticket 005, where an ability was either modeled or unmodeled.
- **Spellcasting.** The stat block keeps a structured block with spell ids. The engine owns a spell effects file keyed by SRD spell id, starting with the 26 damage spells and the combat save and condition spells in the monster blocks, about 40 in all. A spell id not in the file makes the cast an unmodeled ability. The future spells map can replace the file. SRD counts: 90 blocks, 116 distinct spells, 379 spell uses, of which 243 uses are utility (Detect Magic alone is 33).
- **Reaction triggers.** Six: hit by an attack roll (with filters for melee or ranged, wielding a weapon, self or ally), attack roll made against it, takes damage or becomes Bloodied, a creature moves or starts or ends its turn within N feet, a creature makes an ability check or save, a spell is cast. The spell trigger is dormant until a combatant can cast a modeled spell. Any other trigger makes the reaction unmodeled and the agent handles it.
- **Passive triggers.** Six: start of own turn, end of own turn, a creature starts its turn in this emanation, hit by a melee attack, takes damage of a named type, reduced to 0 HP. Anything else is an agent clause.
- **Modifier effect.** One timed modifier covers Speed halved, no Reactions, Disadvantage on a roll kind, an AC bonus and temporary HP. It takes a stat or roll kind, a value or advantage state, and a duration (end of a named turn, a number of rounds, until a save succeeds). It moves most of the 15 failure-branch entries and the 20 dual-breath second breaths into the modeled set.
- **Roll modifier effect.** Gives Advantage, Disadvantage or a flat bonus on a roll kind (attack rolls, saves against spells, ability checks, D20 Tests) under a closed condition list: always, an ally of the attacker within 5 feet of the target, self Bloodied, target below full HP, ally in an aura, took a damage type this turn, and in sunlight or Bright Light. A new encounter setting `lighting` (`normal`, `sunlight`, `bright_light`, default `normal`) feeds the last one. It is a standing assumption in batch and appears in the log.
- **Usage.** Five kinds: `at_will`, `recharge` (minimum 4, 5 or 6 on a d6), `per_day` (count), `per_rest`, `cooldown_until_next_turn`. Legendary Resistance is a `per_day` counter that turns a failed save into a success. Prose-only limits (4 entries) become agent clauses. The import's correction list restores the missing recharge on the 20 dual-breath second breaths.
- **Conditions.** All 15 are modeled with their 2024 effects. Where a condition needs line of sight, combatants see each other unless one is Invisible, Blinded or an override says otherwise. Where it needs distance, the engine uses engagement and the range bands. Each assumption that changes a result goes in the log. The 2024 text of each condition is checked against the rules glossary through the dnd-rules server when the engine is built.
- **Multiattack and references.** `multiattack` is ordered steps, each `{action, count}` or `{choice: [options]}`, with an optional `swap` rule. `use_action {id}` runs another action of the same stat block, which covers legendary actions that mirror an action. `cast {spell_id, usage}` references the spell effects file and uses the usage kinds. Which option or swap to take belongs to the batch tactics policy.
- **Movement.** Forced movement, teleport and move-then-attack reuse the ADR 0003 costs. A push or pull of at least 5 feet starts or ends engagement. A push of 35 feet or more from engaged reaches far, and a pull of 35 feet or more from far reaches engaged. Teleport sets any band its distance covers and provokes no Opportunity Attack. Move-then-attack moves the attacker to engaged with the target and attacks, and an unaffordable move is illegal and says so.
- **Flags.** The 166 flag traits split in two. Those with combat meaning (Magic Resistance, Pack Tactics, Flyby and the Bloodied traits) become `roll_modifier` or movement rules. The rest (Amphibious, Spider Climb, Hold Breath, Illumination) are inert. Batch neither refuses on inert abilities nor flags them in the report.
- **Unmodeled in phase 1.** Persistent zones (10 entries), objects with their own HP (6) and spawn or splitting (17). Revisit after the first reports show how many monsters hit them.
- **Default resolutions.** The SRD import writes a `resolution` onto each unmodeled ability or clause, in three classes. Neutral (revival traits, senses and telepathy, curse and long-term affliction clauses, Wishes, Soul Bag, Spirit Jar and similar out-of-fight rules) counts as no effect in the fight. Base form (28 Shape-Shift and alternate-form entries) keeps the creature in its stat block form. Skipped (spawn and splitting, zones, objects) is skipped, and the report opens with a warning that names each one because they can move win rates. The user can override any default per monster or as a standing assumption. Homebrew abilities get no default, so batch refuses until the agent resolves each one with the user. This refines ticket 005's refusal rule.
- **Survey counts.** The three entries missing from the survey's prose list were Succubus Form, Incubus Form and Half-Dragon Draconic Origin. The survey counted them under Shape-Shift as "plus three", so its total of 83 was right. The survey script's tag table remains a cross-check and not the importer's parser.
- **Surfaced.** The SRD import review leaves the fog and becomes ticket 020, now that the structured side of its comparison has a vocabulary.
