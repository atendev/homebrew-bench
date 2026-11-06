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
Two combatants are engaged when they are in melee with each other. The engine tracks engagement and a coarse range band instead of positions on a map. A rule that says "within 5 feet" reads as engaged.
_Avoid_: Adjacent, in contact

**Sees**:
One combatant sees another unless the other is Invisible and the first has neither Blindsight nor Truesight, or the first is Blinded and has no Blindsight. Sense ranges don't count, and line of sight is otherwise assumed. Invisible's attack effects, the Frightened condition and Opportunity Attacks all use this.
_Avoid_: Line of sight, visible to

**Downed**:
A party member at 0 Hit Points that is dying or stable. For attack rolls it has the Unconscious condition, and so the Prone condition, without either being listed on it.
_Avoid_: Down, knocked out

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
A timed change a modeled ability puts on a combatant's stats or rolls, such as halved Speed, no Reactions, or Disadvantage on Strength checks. It ends on a duration, a successful save, the end of its source's concentration, or, for some, when its bearer takes damage. When one spell puts its modifier on the same bearer more than once, only the most potent applies, or the most recent if they are equal; the others apply again when it ends. A modifier can grant an extra action, and can leave effects on its bearer when it ends, as Haste's lethargy does. Conditions are separate: the ruleset defines them.
_Avoid_: Buff, debuff, status

**Bearer**:
The combatant a modifier is on, whose stats or rolls it changes. A modifier lands on the target of the ability that made it unless the ability marks it as the user's own, as Shield does. The **source** is the combatant whose ability made it; when the source's concentration ends, the modifier ends. Haste cast on an ally has the ally as bearer and the caster as source.
_Avoid_: Owner, holder (a legendary action holder is a different thing), recipient

**Extra action**:
A second action on each of a combatant's turns, granted by a modifier and limited to the uses it lists, such as Haste's one attack or Disengage. It is separate from the combatant's own action and from a bonus action.
_Avoid_: Bonus action (a different thing), additional attack

**Bonus action**:
The one bonus action a combatant may take on its turn, only through an ability that grants one, such as Healing Word or Nimble Escape. It is separate from its action and from an extra action.
_Avoid_: Swift action, minor action

**Roll modifier**:
Advantage or Disadvantage that a trait gives its own creature on one kind of roll while a stated circumstance holds, such as Sunlight Sensitivity in sunlight. It is always on while the circumstance holds, with no duration. Lighting is the only circumstance the engine reads.
_Avoid_: Modifier (that is timed), passive buff

**Lighting**:
The light one encounter takes place in, one value for every combatant: unspecified, Darkness, Dim Light, Bright Light or sunlight. Sunlight is also Bright Light. A trait keyed to Dim Light also applies in Darkness, so it fits the usual "Dim Light or Darkness" wording; a trait that works only in Dim Light is left to the agent. Unspecified is the default and turns on no lighting trait, so a trait keyed to any light applies only when the user names that light. In Darkness, a creature without Darkvision, Blindsight or Truesight sees no one. A party carrying torches fights in Dim Light or Bright Light, not Darkness. Magical Darkness is not a lighting value; the agent handles it.
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

**Buff horizon**:
The rounds the batch tactics policy values a buff over: at most two, no more than the fight is expected to last, with each round weighted by the chance the caster still holds concentration by then. Haste's lethargy counts against it as a lost turn for the bearer, weighted by the chance concentration breaks inside the horizon.
_Avoid_: Buff duration (the spell's own duration is longer)

**Batch tactics policy**:
The deterministic, versioned decision policy that chooses targets, movement, actions, resources, area targets, healing, consumables and reactions for both sides in batch mode. Its decisions and assumptions are logged, and its inputs are part of baseline fingerprints.
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
A pending prompt that offers one combatant a chance to use a reaction, such as Shield or an Opportunity Attack. It holds the move or action that triggered it. When the user (for a PC) or the agent (for a monster) takes or declines the reaction, the engine runs that command again with the answer. Legendary actions have no window.
_Avoid_: Interrupt

**Reaction trigger**:
The event named on a reaction that lets the engine open a reaction window for it. Two triggers open windows: a creature leaving the reactor's reach, which offers the Opportunity Attack every creature has, and being hit by an attack roll, which offers reactions such as Shield and Parry. A reaction with any other trigger is agent judgment.
_Avoid_: Reaction condition, hook

**Reaction reserve**:
What a combatant gives up when it spends its reaction on an Opportunity Attack: the chance to turn a later hit into a miss with Shield or Parry before its next turn. The batch tactics policy makes an Opportunity Attack only when the attack is worth more than the reserve. A combatant with no reaction for hits has no reserve.
_Avoid_: Held reaction, readied reaction (Ready is a different action)

**Legendary window**:
The gap between the end of one creature's turn and the start of the next, when other creatures with legendary actions may take them. The creature whose turn just ended takes none. Each use costs one from the holder's budget for the round, which refills when the holder's own turn starts.
_Avoid_: Legendary phase, off-turn action

**Disengage**:
The step that sets a flag until the actor's turn ends, taken as its action, as a bonus action when an ability such as Nimble Escape grants one, or as an extra action that allows it. Its moves provoke no Opportunity Attack. Flyby and Agile give the same effect permanently, as a trait.
_Avoid_: Withdraw

**Reactor**:
A creature that may take a reaction against another's action or movement. For an Opportunity Attack, it is a creature whose engagement the mover breaks, and the engine resolves its attack before the band changes. For a hit, it is the creature hit, and its reaction resolves before the damage.
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
A filtered read of the log and digest for showing players, not the DM. It hides monster hit points, resources and recharge, and replaces hit points with wounded states. The fact of an override or prompt stays, and the hidden detail goes. A reaction window a monster declined is hidden detail.
_Avoid_: Redacted log, spoiler-free mode

**Wounded state**:
One of four words used in place of a monster's exact hit points in the public view: unhurt at full, wounded below full, bloodied at half or less, at death's door at a quarter or less.
_Avoid_: Health bar, percent health
