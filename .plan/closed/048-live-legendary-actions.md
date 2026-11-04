---
id: 048
title: Live legendary actions
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

How does live mode run legendary actions? Decide the command shape (a new `bench legendary` or `bench action` accepting legendary ids off-turn), when the engine allows one (after another creature's turn ends), how `legendary_uses_per_round` is spent and reset, and whether the engine prompts the agent when a window opens.

## Notes

Stat blocks already carry `legendary_actions` and `legendary_uses_per_round`. The skill currently resolves them with `bench roll` and `bench override`. Check how the batch runner uses legendary actions so both modes share one rule. Consult `grilling` and `domain-modeling`.

## Resolution

Approved by the user on 2026-11-04. The questions and the agreed answers are in [048-grilling-draft](../reports/048-grilling-draft.md). A subagent answered them on its own, and only Q5 needed the user to pick.

- **Window timing.** `end_turn_events` splits into finishing the current turn and starting the next. Legendary actions resolve between the two, as the 2024 rules have it. This also fixes batch: today, a holder that acts next spends from a budget that has already reset, and recharges and death saves roll before the legendary action.
- **Command shape.** Declared on the end-turn, like opportunity attacks in ADR 0015: `bench end-turn --legendary <holder>:<action-id>[=<target>,...]`. The flag repeats and resolves in order. An ineligible declaration fails the whole command. A holder that drops partway loses its remaining uses, and that is logged.
- **Uses per window.** The engine allows several in a row, up to the budget. The batch policy keeps one per window. The engine refuses the creature whose turn just ended, a creature that is down or can't act, one with no uses left, and an action that hasn't recharged, is on cooldown, or is out of PerDay uses. The creature whose turn comes next is allowed, because its budget has not reset yet. `use_legendary_action` goes through `spend_usage`. Today it checks only Recharge.
- **Prompting.** No prompt. A computed `digest.legendary` lists eligible holders, their uses left and their ready actions. The held-EndTurn prompt moves to [Live reaction windows](../tickets/047-live-reaction-windows.md).
- **Batch alignment.** The batch reorder lands in the build. `ENGINE_VERSION` goes up, `POLICY_VERSION` stays at 2, and the baseline is not regenerated, so it reports itself stale through `Baseline::stale_reasons`. The one regeneration still happens with the v3 build.
- **Log and undo.** `LEGENDARY ACTION <holder>: <name> (<used> of <budget>)` goes between the `end_turn` and `start_turn` lines. Undo reverts the whole end-turn. Replay keeps the list in the command arguments, and a bare `EndTurn` still deserializes.
- **PC dice in ask mode.** The whole end-turn holds, and `respond` reruns it from the state before the command, so durations don't tick down twice.
- **Limits, not built.** Lair action use counts, legendary actions that cost more than one use, and legendary actions that move.
- Also: one window per end-turn even when skipped combatants are passed over, the window resolves before the round increments, a new ADR or an amendment to ADRs 0004 and 0012, and an update to `live-mode.md` line 50.
- Build ticket: [Build live legendary actions](../tickets/057-build-live-legendary-actions.md).
