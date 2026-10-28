# Homebrew Bench

A tool for testing D&D 5e homebrew by playing out combats and measuring the numbers, with an AI agent doing the interpretation and the DM role.

## Language

**Homebrew**:
Custom content written by a user that is not in the published rules. Phase 1 covers monsters only.
_Avoid_: Custom content, mod

**Engine**:
The code that holds the true state of a fight and does the rules math. It is the source of truth for combat, except where the agent overrides it.
_Avoid_: Simulator, game, backend

**Agent**:
The AI that reads homebrew, drives the engine, and reports or narrates results. It is the only party that makes judgment calls.
_Avoid_: Bot, assistant, LLM

**Batch mode**:
The agent converts homebrew, runs many fights through the engine, and writes a report of numbers and wording problems.
_Avoid_: Simulation mode, bench run

**Live mode**:
The agent acts as DM for one combat encounter, passing each action through the engine one step at a time while the user controls the party.
_Avoid_: Play mode, interactive mode

**Encounter**:
One combat the engine holds state for, from rolling initiative until it ends. Live mode plays one encounter turn by turn. Batch mode plays many encounters inside a single engine call.
_Avoid_: Fight, battle, session

**Stat block**:
The definition of a creature, SRD or homebrew, in our own format. Party members use stat blocks too.
_Avoid_: Monster file, creature (a rules term in 5e)

**Combatant**:
One creature taking part in an encounter, created from a stat block and carrying its own state. Four goblins from one stat block are four combatants.
_Avoid_: Creature, unit, token

**Engaged**:
Two combatants are engaged when they are in melee with each other. The engine tracks engagement and a coarse range band instead of positions on a map.
_Avoid_: Adjacent, in contact

**Modeled ability**:
An ability the engine can run from its structured effects. Every other ability is an **unmodeled ability**: it keeps only its text, and the agent resolves it as agent judgment.
_Avoid_: Supported, custom ability

**Agent clause**:
A part of an otherwise modeled ability that the engine cannot run, kept as text for the agent to resolve. The rest of the ability still runs through the engine.
_Avoid_: Rider, partial ability

**Modifier**:
A timed change a modeled ability puts on a combatant's stats or rolls, such as halved Speed, no Reactions, or Disadvantage on Strength checks. It ends on a duration or a successful save. Conditions are separate: the ruleset defines them.
_Avoid_: Buff, debuff, status

**Agent judgment**:
A call the engine cannot make, such as an ambiguous or unmodelable ability, made by the agent and clearly labeled as such in reports and in play.
_Avoid_: Fudge, guess

**Override**:
An agent action that sets any part of the engine's state directly, such as health or damage, instead of letting the engine compute it.
_Avoid_: Patch, hack

**Standing assumption**:
An override the agent sets at intake in batch mode so the engine applies it in every run, labeled as an assumption in the report.
_Avoid_: Default

**Ruleset**:
The rules the engine models: the combat rules of SRD 5.2, the 2024 edition. Anything outside it is not modeled unless a house rule or agent judgment covers it.
_Avoid_: Edition, rules version

**House rule**:
A rule the user adds beyond the ruleset, either to fill a gap or to change how a ruleset rule works. It persists across runs, the agent applies it, and the log names it when used.
_Avoid_: Homebrew rule, custom rule

**Rule slug**:
The name of a house rule, taken from its file name. The log tags every outcome a house rule changed with its slug.
_Avoid_: Rule id, rule number

**Fixture**:
An SRD stat block that has cleared import review and ships as trusted reference data. The engine only ever sees fixtures; a stat block with open mismatches stays unavailable until they are fixed. Fixtures graduate one monster at a time.
_Avoid_: Validated monster, gold master

**Correction**:
A checked-in patch to an upstream SRD data fault, applied by the import instead of editing upstream data. Each correction names the monster, the field, the corrected value, and a citation. Every applied correction appears on the import review list.
_Avoid_: Fix, patch, override of import data

**Import review list**:
The generated list the import writes each run: mismatches between upstream structured fields and the prose or ruleset tables, plus one classification note per ability recording where it landed (modeled, partly modeled, inert, unmodeled, and its default resolution). Only mismatches block a stat block from becoming a fixture.
_Avoid_: Diff report, error log

**Pending prompt**:
A pause in a live encounter where the engine waits for a decision or a die roll before it finishes the current action. Nothing else changes the encounter until it is answered.
_Avoid_: Interrupt, callback

**Reaction window**:
A pending prompt that offers one combatant a chance to use a reaction, such as Shield or an Opportunity Attack, or a legendary action between turns.
_Avoid_: Trigger, interrupt

**Seed**:
The number that fixes an encounter's dice stream. Every encounter has one. Batch encounters derive theirs from the batch's master seed and fight index. The same seed and the same commands give the same dice.
_Avoid_: Random state, entropy

**Command history**:
The ordered list of mutating commands a live encounter has processed, stored with their arguments and any user-entered dice. Replay re-executes it from the initial snapshot.
_Avoid_: Event log, transcript

**Replay**:
Re-running a live encounter's command history from its seed to rebuild the state, as a tamper check or to find where an old encounter diverges under newer rules.
_Avoid_: Resume, restore

**Log**:
The human-readable record of every action the engine takes, including overrides with their stated reasons. The log cannot be overridden.
_Avoid_: History, trace

**Public view**:
A filtered read of the log and digest for showing players, not the DM. It hides monster hit points, resources and recharge, and replaces hit points with wounded states. The fact of an override or prompt stays; the hidden detail goes.
_Avoid_: Redacted log, spoiler-free mode

**Wounded state**:
One of four words used in place of a monster's exact hit points in the public view: unhurt at full, wounded below full, bloodied at half or less, at death's door at a quarter or less.
_Avoid_: Health bar, percent health
