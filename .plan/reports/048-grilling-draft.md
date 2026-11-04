# Live legendary actions: draft decisions (approved 2026-11-04)

Questions and recommendations came from the parent session. A subagent answered them independently, and this file records where the two agree. The user approved these on 2026-11-04.

1. **Window timing.** Split `end_turn_events` into finishing the current turn and starting the next one. Legendary actions resolve between the two, which matches the 2024 rule. This fixes a batch ordering bug: today a holder that goes next spends from a budget that has already reset, and recharges and death saves roll before the legendary action does.
2. **Command shape.** Declare on end-turn, the same way ADR 0015 declares opportunity attacks: `bench end-turn --legendary <holder>:<action-id>[=<target>,...]`. The flag repeats and resolves in order. An ineligible declaration fails the whole command. A holder that goes down partway through loses the uses it has left, and the event is logged.
3. **Uses per window.** The engine allows several uses in a row, up to the budget. The batch policy keeps one per window. The engine refuses the creature whose turn just ended, a creature that can't act or is down, a creature with no uses left, and an action that has not recharged, is on cooldown, or has no PerDay uses left. The creature whose turn comes next is allowed, because its budget has not reset yet. Route `use_legendary_action` through `spend_usage`. Today it checks only Recharge.
4. **Prompting.** No prompt. A computed `digest.legendary` lists eligible holders, their uses left, and their ready actions. The held-EndTurn prompt moves to Live reaction windows.
5. **Batch alignment.** User picked: land the batch reorder in the build, bump `ENGINE_VERSION`, keep `POLICY_VERSION` at 2, and let the baseline report itself stale.
6. **Log and undo.** Log `LEGENDARY ACTION <holder>: <name> (<used> of <budget>)` between the `end_turn` and `start_turn` lines. Undo reverts the whole end-turn. Replay stores the list in the command arguments. A bare `EndTurn` must still deserialize.
7. **PC dice in ask mode.** The whole end-turn holds, and `respond` reruns it from the state before the command, so durations are not decremented twice.
8. **Limits, not built.** Lair use counts, legendary actions that cost more than one use, and legendary actions that move.

Also needed: one window per end-turn even when skipped combatants are passed over, the window resolves before the round increments, a new ADR or amendment to ADR 0004 and ADR 0012, and an update to `live-mode.md` line 50.
