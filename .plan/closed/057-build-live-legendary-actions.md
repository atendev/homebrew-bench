---
id: 057
title: Build live legendary actions
type: task
mode: AFK
status: closed
blocked_by: [048]
claimed: false
---

## Question

Build what [Live legendary actions](../closed/048-live-legendary-actions.md) decided: a legendary window between the end of one turn and the start of the next, declared with `bench end-turn --legendary <holder>:<action-id>[=<target>,...]`, shared by live and batch.

## Notes

- Split `end_turn_events` (`bench-core/src/engine.rs`) into finishing the current turn and starting the next. Resolve the window between them, before the round increments, once per end-turn even when skipped combatants are passed over. Move the batch `legendary_window` (`bench-core/src/batch.rs`) into that gap.
- Route `use_legendary_action` through `spend_usage`, so Recharge, CooldownUntilNextTurn and PerDay all apply. The eligibility rules are in the 048 Resolution. Use one eligibility function for the CLI and batch.
- An ineligible declaration fails the whole command. A holder that drops partway loses its remaining uses, and that is logged.
- Add a computed `digest.legendary` (eligible holders, uses left, ready actions). There is no prompt.
- Log `LEGENDARY ACTION <holder>: <name> (<used> of <budget>)` between `end_turn` and `start_turn`. Undo reverts the whole end-turn. Replay keeps the list in the command arguments, and a bare `EndTurn` must still deserialize.
- In ask mode, PC dice hold the whole end-turn, and `respond` reruns it from the state before the command.
- Batch: bump `ENGINE_VERSION`, keep `POLICY_VERSION` at 2, and leave one use per window. Do not regenerate `data/baseline/curve.json`. It reports itself stale through `Baseline::stale_reasons`, and the v3 build ([056](056-build-v3-scoring-for-buffs-forced-movement-and-teleports.md)) regenerates it once.
- Docs: a new ADR or amendments to ADRs 0004 and 0012, and an update to `.agents/skills/homebrew-bench/references/live-mode.md` line 50. The skill test checks the new flag against `bench help --json`.
- Known limits, not built: lair action use counts, legendary actions that cost more than one use, and legendary actions that move.
- Consult `tdd`, `codebase-design`, `domain-modeling` and `code-review`.

## Resolution

Built on branch `ticket-057-build-live-legendary-actions`.

- `end_turn_events` is split into `finish_turn` and `start_next_turn`, both public. The legendary window resolves between them, before the round advances, once per end-turn. Batch calls the same split and runs its policy in the gap, one use per holder per window.
- `StepRequest::EndTurn` and `Command::EndTurn` carry a `legendary` list of `LegendaryUse { holder_id, action_id, target_ids }`. An empty list is not serialized, so a bare end-turn reads and writes as before. A dice prompt saved with a bare `"end_turn"` request still loads.
- `bench end-turn --legendary <holder>:<action-id>[=<target>,...]` repeats and resolves in order.
- `legendary_uses_left` is the one eligibility rule. The CLI, batch and `digest.legendary` all use it. It refuses the creature whose turn is ending, one that can't act, and one with no uses left. The creature up next is allowed. `use_legendary_action` spends through `spend_usage`, so Recharge, CooldownUntilNextTurn and PerDay apply. Recharge legendary actions now roll at the holder's turn start.
- Every declaration is checked on a scratch copy before anything resolves. A failure refuses the whole command and changes nothing. A holder that drops partway logs `legendary_lost` and loses its remaining uses.
- Each use logs `LEGENDARY ACTION <holder>: <name> (<used> of <budget>)`. The public view drops the count for monsters. `digest.legendary` lists holders, uses left and ready actions, and the public digest leaves it out.
- Ask mode holds the whole end-turn and `respond` reruns it from the state before the command. Undo reverts the end-turn and its legendary actions together. Replay keeps the list in the stored arguments.
- The workspace version, which is `ENGINE_VERSION`, went from 0.1.0 to 0.1.1. A patch bump keeps 0.1.0 live encounters loadable, because an end-turn without `--legendary` resolves as before. `POLICY_VERSION` stays 2. The baseline was not regenerated, and `bench baseline status` reports only the engine version as stale. The calibration and CLI tests now allow that one reason. Ticket 056 regenerates the curve once and removes the allowances. Ticket 058 widens them to the fixture-set reason in the meantime.
- Batch numbers change for monsters with modeled legendary actions. An adult white dragon over 40 fights at levels 10 to 14 went from 40/36/21/16/9 to 39/31/25/17/13 party losses. Monsters without legendary actions are unchanged.
- ADR 0016 is written, and ADRs 0004 and 0012 are amended. `live-mode.md` replaces the old line 50 with a step for declaring legendary actions. `CONTEXT.md` adds **Legendary window** and drops legendary actions from **Reaction window**. `CHANGELOG.md` is updated.
- Tests cover the window timing, the next creature's budget, six refusal cases, all three usage kinds, a holder dropping partway, skipped creatures and the round increment, the holder list, legendary recharge, undo, replay, the ask-mode hold, the bare end-turn, the public line, the CLI flag, and the batch order.
- The repo is not rustfmt-clean on master (about 13,000 lines of drift), so `cargo fmt` was not run across it. Clippy with `-D warnings` fails on five findings that already exist on master. This build adds none.
- New: [Import legendary action cooldowns](058-import-legendary-action-cooldowns.md). 44 SRD legendary actions say they can't be used again until the start of the holder's next turn, and the importer leaves their usage empty.
