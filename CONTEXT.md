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

**Cast level**:
The level a spell takes on for one casting: the level of the slot spent, or the spell's own level when no slot is named. A monster casts at the spell's own level unless its stat block names a "level N version". Cantrips have no cast level; their damage grows with the caster's character level instead.
_Avoid_: Spell level (that is the spell's fixed level), slot

**Upcast**:
A cast whose cast level is above the spell's own level. Scaling rules in the spell effects file say what grows per level. What the engine can't apply is kept as an upcast clause, a kind of agent clause.
_Avoid_: Overcast, higher-level casting

**Modifier**:
A timed change a modeled ability puts on a combatant's stats or rolls, such as halved Speed, no Reactions, or Disadvantage on Strength checks. It ends on a duration or a successful save. Conditions are separate: the ruleset defines them.
_Avoid_: Buff, debuff, status

**Roll modifier**:
Advantage or Disadvantage that a trait gives its own creature on one kind of roll while a stated circumstance holds, such as Sunlight Sensitivity in sunlight. It is always on while the circumstance holds, with no duration. Lighting is the only circumstance the engine reads.
_Avoid_: Modifier (that is timed), passive buff

**Lighting**:
The light one encounter takes place in, one value for every combatant: Dim Light, Bright Light or sunlight. Sunlight is also Bright Light. Dim Light is the default, so no lighting trait applies unless the user picks brighter light. Darkness is not a lighting value; the agent handles it.
_Avoid_: Normal light, illumination, ambient light

**Agent judgment**:
A call the engine cannot make, such as an ambiguous or unmodelable ability, made by the agent and clearly labeled as such in reports and in play.
_Avoid_: Fudge, guess

**Override**:
An agent action that sets any part of the engine's state directly, such as health or damage, instead of letting the engine compute it.
_Avoid_: Patch, hack

**Standing assumption**:
An override the agent sets at intake in batch mode so the engine applies it in every run, labeled as an assumption in the report.
_Avoid_: Default

**Standard party**:
The built-in party of four stat blocks, one per role (defender, striker, healer, caster), available at every level from 1 to 20. Batch mode fights homebrew against it by default, and `--party` substitutes a user's own party. It carries no magic items.
_Avoid_: Default party, test party

**Batch tactics policy**:
The deterministic, versioned decision policy that chooses targets, movement, actions, resources, area targets, healing and consumables for both sides in batch mode. Its decisions and assumptions are logged, and its inputs are part of baseline fingerprints.
_Avoid_: AI strategy, simulator behavior

**Ruleset**:
The rules the engine models: the combat rules of SRD 5.2.1, the 2024 edition. Anything outside it is not modeled unless a house rule or agent judgment covers it.
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

**Baseline curve**:
The committed results of running every fixture against the standard party at each party level under the batch tactics policy. It is the reference distribution for judging a single homebrew monster, not an SRD table.
_Avoid_: CR table, benchmark table

**Difficulty band**:
The calibrated Low, Moderate or High reading assigned to a fight from its party win rate and death rate. The thresholds are homebrew-bench calibration, not SRD numbers.
_Avoid_: Difficulty score, rating

**Effective CR**:
The lowest whole party level at which a monster reads as a Low fight against the baseline curve. Reports clamp results below level 1 to around 1 or trivial and results above level 20 to over 20.
_Avoid_: True CR, recommended CR

**Thin data**:
A baseline cell with too few fixtures even after pooling the neighboring CRs. It is named in reports rather than silently smoothed.
_Avoid_: Missing data, interpolated data

**Casualty**:
A PC death during a batch encounter. A PC being downed but later saved is not a casualty for difficulty-band calibration.
_Avoid_: Down, knockout

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
A pending prompt that offers one combatant a chance to use a reaction, such as Shield or an Opportunity Attack.
_Avoid_: Trigger, interrupt

**Legendary window**:
The gap between the end of one creature's turn and the start of the next, when other creatures with legendary actions may take them. The creature whose turn just ended takes none. Each use costs one from the holder's budget for the round, which refills when the holder's own turn starts.
_Avoid_: Legendary phase, off-turn action

**Disengage**:
The standard action that spends the actor's action and sets a flag until its turn ends. Its moves provoke no Opportunity Attack. Flyby and Agile give the same effect permanently as the `no_provoke` trait effect.
_Avoid_: Withdraw

**Reactor**:
A creature that takes a reaction against another's action or movement. For an Opportunity Attack, it is a creature whose engagement the mover breaks. The move names its reactors, and the engine resolves their attacks before the band changes.
_Avoid_: Responder, defender

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
