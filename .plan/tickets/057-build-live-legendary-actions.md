---
id: 057
title: Build live legendary actions
type: task
mode: AFK
status: open
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
