---
id: 047
title: Live reaction windows
type: grilling
mode: HITL
status: open
blocked_by: [044]
claimed: false
---

## Question

Should live mode open reaction windows when a trigger fires, so the engine prompts for a PC's reaction and names each monster that may react, instead of leaving the agent to notice triggers? Decide which of the six reaction triggers open a window, whether a window is a pending prompt that blocks the next command, and how declining one is recorded in the log and for replay.

## Notes

Today the skill tells the agent to ask the user about reactions itself (`.agents/skills/homebrew-bench/references/live-mode.md`, Reactions). Opportunity attacks follow ticket 044's answer. Pending prompts and undo follow ADR 0004. Consult `grilling` and `domain-modeling`.

## Resolution
