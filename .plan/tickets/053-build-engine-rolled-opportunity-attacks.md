---
id: 053
title: Build engine-rolled opportunity attacks
type: task
mode: AFK
status: open
blocked_by: []
claimed: true
---

## Question

Build the decision in [Engine-rolled opportunity attacks](../closed/044-engine-rolled-opportunity-attacks.md): a shared eligibility function used by the event and the resolver, an `opportunity` list on `StepRequest::Move`, and `bench move --opportunity <reactor>[:<attack-id>]...`.

## Notes

Tests cover reaction spent on hit or miss, reset on the owner's turn, a downed mover not finishing the move or paying feet, an ineligible named reactor failing the command, the ask-mode dice prompt, undo, and replay. Update `.agents/skills/homebrew-bench/references/live-mode.md` and the `bench help --json` docs (the skill test checks them). Write ADR 0015, "Opportunity attacks are declared on the move", and amend ADR 0004's Opportunity Attack clause: until ticket 047 adds reaction windows the agent names reactors by judgment. Batch results and `POLICY_VERSION` stay unchanged. Consult `tdd`, `codebase-design`, `domain-modeling`.

## Resolution
