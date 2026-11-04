---
id: 047
title: Live reaction windows
type: grilling
mode: HITL
status: open
blocked_by: [053]
claimed: false
---

## Question

Should live mode open reaction windows when a trigger fires, so the engine prompts for a PC's reaction and names each monster that may react, instead of leaving the agent to notice triggers? Decide which of the six reaction triggers open a window, whether a window is a pending prompt that blocks the next command, and how declining one is recorded in the log and for replay.

## Notes

Today the skill tells the agent to ask the user about reactions itself (`.agents/skills/homebrew-bench/references/live-mode.md`, Reactions). Opportunity attacks follow ticket 044's answer. Pending prompts and undo follow ADR 0004. Consult `grilling` and `domain-modeling`.

Scope added by [Engine-rolled opportunity attacks](../closed/044-engine-rolled-opportunity-attacks.md): an opportunity-attack window is a held `Move` request whose prompt asks which listed reactors attack, and its answer fills the `opportunity` list. The build waits on the opportunity-attack build ticket.

Scope added by [Live legendary actions](../closed/048-live-legendary-actions.md): a legendary window could be a held `EndTurn` request whose prompt lists eligible holders and their ready actions, and its answer fills the `legendary` list. Phase 1 has no prompt, only `digest.legendary`, so decide whether this window opens alongside the reaction windows.

## Resolution
