# Legendary actions are declared on the end-turn

Live mode had no command for legendary actions. The agent resolved them with `bench roll` and `bench override`, so the engine never checked the per-round budget or the action's own usage. Batch ran its own legendary window after `EndTurn` had already started the next turn. A holder whose turn came next spent from a budget that had just reset, and its recharge rolls and death saves came before the legendary action instead of after. ADR 0004 lists legendary actions among the reaction windows, and ADR 0012 says they are spent "in the current eligible window" without saying where that window is.

The legendary window sits between the end of one turn and the start of the next, as the 2024 rules have it. The engine splits the end-turn into finishing the current turn and starting the next, and resolves legendary actions between the two. The window opens once per end-turn, even when the next turn passes over creatures that are down, and it resolves before the round advances.

- **Command.** Live mode declares the window on the end-turn, the way ADR 0015 declares Opportunity Attacks on the move: `bench end-turn --legendary <holder>:<action-id>[=<target>,...]`. The flag repeats and resolves in the order given. `StepRequest::EndTurn` and the live `Command::EndTurn` carry the list. An empty list is left out of the stored command, so a bare end-turn reads and writes as before.
- **Eligibility.** One function decides for the live command, the batch policy and the digest. It refuses the creature whose turn is ending, a creature that can't act, and one with no uses left this round. The creature whose turn comes next is allowed, because its budget resets only when its turn starts. Each use costs one from the budget and also spends the action's own usage, so Recharge, CooldownUntilNextTurn and PerDay all apply. Recharge legendary actions roll at the holder's turn start like any other.
- **Refusal.** Every declaration is checked before anything resolves. One that fails refuses the whole command with the reason, and nothing changes. A holder that drops partway through the window, for example to another holder's legendary action, loses its remaining declared uses. That is logged as `legendary_lost` and does not fail the command.
- **No prompt.** The engine does not pause when a window opens. The DM digest carries `legendary`: each creature that may act when the active turn ends, its uses left and its ready actions. The held end-turn prompt belongs to the live reaction windows ticket (047).
- **Log.** Each use logs `LEGENDARY ACTION <holder>: <name> (<used> of <budget>)` between the `end_turn` and `start_turn` lines. The public view drops the count for monsters.
- **Batch.** The batch runner calls the same split and runs its policy in the gap. It still takes at most one use per holder per window. `ENGINE_VERSION` went from 0.1.0 to 0.1.1 and `POLICY_VERSION` stays 2. The baseline curve was not regenerated, so it reports itself stale through `Baseline::stale_reasons` until the v3 build (ticket 056) regenerates it once.

## Considered options

- **A separate `bench legendary` command, or `bench action` accepting legendary ids off-turn.** Rejected. Nothing ties the action to a window, so it could run mid-turn, and undo would revert it apart from the end-turn that opened the window.
- **A prompt that holds every end-turn when a window opens.** Rejected for now. Most windows go unused, and a prompt on each would double the commands per turn. The digest gives the agent the same facts.
- **Keep the batch window after the full end-turn.** Rejected. It spends from a budget that has already reset and lets start-of-turn rolls run first.
- **Regenerate the baseline in this build.** Rejected. The v3 scoring build changes the policy too, and one regeneration covers both.

## Consequences

- One end-turn is one logged unit. `bench undo` reverts the turn change and every legendary action in it. Replay needs no new command kind because the list is part of the stored arguments.
- Under `pc_dice = ask`, PC dice in the window hold the whole end-turn on a dice prompt. `bench respond` reruns it from the state before the command, so durations tick down once.
- A dice prompt saved before this change holds a bare `"end_turn"` request, and it still loads.
- Live encounters saved by engine 0.1.0 still load. Without `--legendary`, an end-turn resolves as before, except that a Recharge legendary action now rolls at its holder's turn start.
- Not built: lair action use counts, legendary actions that cost more than one use, and legendary actions that move the holder.
