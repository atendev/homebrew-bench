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

**Log**:
The human-readable record of every action the engine takes, including overrides with their stated reasons. The log cannot be overridden.
_Avoid_: History, trace
