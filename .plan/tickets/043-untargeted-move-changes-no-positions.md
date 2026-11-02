---
id: 043
title: Untargeted move changes no positions
type: task
mode: AFK
status: open
blocked_by: []
claimed: false
---

## Question

Make `bench move <actor> <band>` with no `--target` move the actor relative to every other combatant, as `bench help --json` describes, including breaking engagement and raising `opportunity_attack` events.

## Notes

`move_combatant` in `bench-core/src/engine.rs` updates `ranges` and `engagement` only inside `if let Some(target_id)`. With no target it spends movement and logs the move but leaves every position unchanged: a combatant engaged with an Ogre stays engaged after `bench move ogre far`. Decide the cost rule for a move relative to everyone (the largest single-pair cost is the likely answer) and whether the batch tactics policy depends on the current behavior. Once fixed, drop the "always pass `--target`" workaround from `.agents/skills/homebrew-bench/references/live-mode.md`.

## Resolution
