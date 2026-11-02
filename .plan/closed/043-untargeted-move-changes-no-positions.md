---
id: 043
title: Untargeted move changes no positions
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Make `bench move <actor> <band>` with no `--target` move the actor relative to every other combatant, as `bench help --json` describes, including breaking engagement and raising `opportunity_attack` events.

## Notes

`move_combatant` in `bench-core/src/engine.rs` updates `ranges` and `engagement` only inside `if let Some(target_id)`. With no target it spends movement and logs the move but leaves every position unchanged: a combatant engaged with an Ogre stays engaged after `bench move ogre far`. Decide the cost rule for a move relative to everyone (the largest single-pair cost is the likely answer) and whether the batch tactics policy depends on the current behavior. Once fixed, drop the "always pass `--target`" workaround from `.agents/skills/homebrew-bench/references/live-mode.md`.

## Resolution

`bench move <actor> <near|far>` with no `--target` now moves the actor to that band relative to every other combatant.

- Each pair gets the new band on both sides, and every engagement the move leaves is broken.
- Cost is the largest single-pair band change among combatants whose `end_state` is `Active`. Each active combatant the move unengages gets an `opportunity_attack` event. Dead, unconscious and stable combatants are still repositioned and unengaged, but they neither add to the cost nor get an event. Nothing clears engagement on death, so leaving them out entirely would leave stale engagement that comes back on healing (audit round 2).
- An untargeted move to `engaged` is refused: engaging everyone at once is not a position the abstract model should produce. `--target` help now says it is required for `engaged`.
- The batch tactics policy never sends an untargeted move (`batch.rs` always passes `Some(target)`). The targeted path only moved into shared `band_change_cost` and `set_band` helpers, so batch output and the baseline curve are unchanged.
- The "always pass `--target`" workaround is gone from `live-mode.md`. `CHANGELOG.md` records the change and a replay note: encounters that used an untargeted move diverge at that move, or fail there if it was to `engaged`.
- Tests: four engine unit tests cover the everyone-move, the largest-pair cost, downed combatants, and the `engaged` refusal. `cargo test --workspace` passes; clippy warnings unchanged.
- Audit: haiku, three rounds. Round 1 found downed creatures inflating cost and getting events; round 2 found the round-1 fix left stale engagement; round 3 approved.
