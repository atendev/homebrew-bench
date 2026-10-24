---
id: 007
title: Live mode turn contract
type: grilling
mode: HITL
status: closed
blocked_by: [004]
claimed: false
---

## Question

In live mode, what does one engine command cover, and who rolls the dice? Does the agent submit one creature action that the engine rolls and resolves, or does it step through attack, damage and saves separately? Can the user roll physical dice and enter the results? How do turns and rounds advance, and how does the agent stay honest to the engine's state while narrating?

## Notes

Graduated from the fog item "the contract between the agent-as-DM and the engine in live mode". The interface is fixed by "How the agent talks to the engine": a one-shot CLI with JSON on stdout, a state file per encounter, a tamper check, and a `bench override` command that requires a reason. Override path syntax belongs to the override and log fog item, not here. "Monster schema and engine state model" fixed one step function that live mode calls once per command, and left this ticket to decide what one step covers and how reaction windows open. Consult `grilling` and `domain-modeling`.

## Resolution

Decided with the user on 2026-10-24. Recorded in `docs/adr/0004-whole-actions-with-pending-prompts.md`. New terms (pending prompt, reaction window) are in `CONTEXT.md`.

- **One command, one action.** The agent declares an action and the engine resolves every roll and effect inside it. The engine splits an action only when it pauses for a decision.
- **Pending prompts.** One mechanism covers reaction decisions, Opportunity Attacks, legendary actions between turns, and user-entered dice. The prompt is saved in the state file and blocks everything except `bench respond`, `bench undo`, `bench override` and read-only commands. A PC's decisions go to the user and a monster's go to the agent. Windows open only for modeled reactions, Opportunity Attacks and legendary actions, and only when the combatant has its reaction unspent and a matching trigger. Shield-type reactions open after the hit is known. The agent handles unmodeled reactions through an override.
- **Dice.** The engine rolls everything by default. Monster rolls are always the engine's. With the encounter setting `pc_dice = engine | ask`, `ask` makes a PC's rolls (attacks, damage, initiative, death saves) pause on a prompt. The user enters natural die faces, and the engine applies modifiers, advantage and crits, rejects out-of-range faces, and marks each roll as engine-rolled or user-entered in the log.
- **PC actions** go through the engine like monster actions. The agent translates the user's words into the same action command.
- **Turns.** `bench end-turn` advances. It runs end-of-turn and start-of-turn effects, skips dead, unconscious and stable combatants, rolls recharge, repeat saves and death saves, increments the round on wrap, and logs a forfeited action. The engine never advances on its own.
- **Honest narration.** Every response carries `events` (roll, outcome, quotable log line), a `digest` of the state after the command, and `pending`. `bench state` returns the same digest at any time. The agent states no mechanical outcome absent from those; anything else is labeled agent judgment and goes through `bench override` with a reason.
- **Visibility.** The user sees all numbers by default. An optional config setting turns on a player-facing public view that hides monster HP numbers and similar detail.
- **Mistakes.** `bench undo` reverts one command. The log keeps both the action and the reversal. Larger corrections are overrides with a reason.
- **Rolls outside the rules.** `bench roll <expression> --reason <text>` logs the result and labels it agent judgment. Seeds and replay stay in their own fog item.
- **Start.** `bench start` takes combatants, sides and starting bands. Everyone starts at `near` unless the agent names bands. Initiative follows the state-model rule, and with `pc_dice = ask` the user rolls PC initiative. Surprise is a condition the agent names per combatant, and the engine applies the 2024 initiative disadvantage only. It returns the first digest.
- **End.** The engine checks the end condition after every command and returns `ended: true` with the winning side, then accepts only read-only commands. Fleeing, surrender and abandoning an encounter go through `bench end --reason <text>`, logged as agent judgment.
- **Possible addition, not built.** An engine-played monster turn (`bench auto-turn`, calling the batch tactics policy) can be added later over the same step function without changing this contract. The map lists it under Out of scope.
- **Surfaced.** The contents of the optional public view became a new fog item.
