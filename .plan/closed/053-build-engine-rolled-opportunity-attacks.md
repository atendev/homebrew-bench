---
id: 053
title: Build engine-rolled opportunity attacks
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Build the decision in [Engine-rolled opportunity attacks](../closed/044-engine-rolled-opportunity-attacks.md): a shared eligibility function used by the event and the resolver, an `opportunity` list on `StepRequest::Move`, and `bench move --opportunity <reactor>[:<attack-id>]...`.

## Notes

Tests cover reaction spent on hit or miss, reset on the owner's turn, a downed mover not finishing the move or paying feet, an ineligible named reactor failing the command, the ask-mode dice prompt, undo, and replay. Update `.agents/skills/homebrew-bench/references/live-mode.md` and the `bench help --json` docs (the skill test checks them). Write ADR 0015, "Opportunity attacks are declared on the move", and amend ADR 0004's Opportunity Attack clause: until ticket 047 adds reaction windows the agent names reactors by judgment. Batch results and `POLICY_VERSION` stay unchanged. Consult `tdd`, `codebase-design`, `domain-modeling`.

## Resolution

Built on branch `ticket-053-build-engine-rolled-opportunity-attacks` (commit ddbbd49). A Haiku audit approved it on the second round.

- `StepRequest::Move` and the live `Command::Move` gain an `opportunity` list of `OpportunityAttack { reactor_id, attack_id }`. Both are serde-default, so stored histories still load. `bench move --opportunity <reactor>[:<attack-id>]` is repeatable, and `bench help --json` picks it up from clap.
- `opportunity_eligibility` in `engine.rs` decides both the `opportunity_attack` notice and the resolver. It checks: opposing side, `can_act`, no `Reactions` modifier sourced on the reactor, reaction unused, mover not Invisible, reactor not Blinded. A sight failure is logged as an `assumption` event. The notice no longer says "has no reaction available" and never names allies.
- Every named reactor is validated before any attack rolls: provoked, eligible, not named twice, has a qualifying attack, and has a use left for a limited attack. Any failure fails the whole move and leaves state untouched.
- `opportunity_attack_action` picks the first at-will single melee attack, walking Multiattack steps (Choice options in order, Alternatives sets, cast steps skipped). A named id may be limited-use and spends through `spend_usage`, which is now shared with `resolve_action`.
- Each attack logs `OPPORTUNITY ATTACK <reactor> on <mover> (reaction)` before its attack lines and spends the reaction, hit or miss. If the mover can't act afterward, a `move_stopped` event ends the command with no band change and no feet charged.
- Tests cover the attack firing before the band change, a miss spending the reaction, the reset on the reactor's turn, a downed mover, eight ineligible-reactor cases, notices, default attack selection, limited-use spend, a spent use failing before anything resolves, an ask-mode PC dice prompt holding the move, and undo plus replay.
- ADR 0015 is written and ADR 0004's Opportunity Attack clause is amended. `live-mode.md`, `CONTEXT.md` (**Reactor**) and `CHANGELOG.md` (Unreleased) are updated. Batch passes an empty list, `POLICY_VERSION` stays 2, and `bench baseline status` reports no stale inputs.
- Left for Build Disengage and no-provoke traits: the no-provoke check goes in `opportunity_eligibility`. ADR 0015 records that it is missing.
