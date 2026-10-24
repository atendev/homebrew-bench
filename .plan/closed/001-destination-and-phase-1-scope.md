---
id: 001
title: Destination and phase 1 scope
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

What does reaching the end of this map look like, for both batch testing and live play with the agent as DM? Which homebrew types and which rules are in scope for phase 1?

## Notes

The map's Destination was a draft written from the user's description: one engine, two modes. This ticket confirmed and rewrote it.

## Resolution

Decided with the user on 2026-10-24.

- Audience: DMs first, homebrew designers second.
- Phase 1 covers monsters only, in both modes. Both modes ship in phase 1.
- Delivery: one engine plus an agent-agnostic skill, which is plain instructions that teach any LLM how to use the engine. The interface between them is a later ticket.
- Batch mode is done when pasted monster text becomes a report with win rate, rounds, and damage per round across party levels, compared with CR guidance, plus wording problems found at intake. Report format details are deferred.
- Live mode is done when the agent DMs one full combat encounter turn by turn. The user controls the whole party and the agent controls the monsters. Only combat and numbers are tested; story, social, and exploration play are out.
- Anything the engine can't model is agent judgment, clearly labeled in reports and in play.
- Overrides: the agent can override nearly any engine state. Batch mode uses standing assumptions set at intake. Live mode uses free-form mid-fight overrides.
- Every engine action goes into a human-readable log, overrides included with their reasons. The log is the only thing that cannot be overridden.
- This map covers phase 1 only. Items, spells, classes, subclasses, and the prose rules reference are separate future maps. The design should not block them.
- Glossary terms live in `CONTEXT.md`.
