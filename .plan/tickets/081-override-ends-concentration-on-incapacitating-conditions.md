---
id: 081
title: Override ends concentration on incapacitating conditions
type: task
mode: AFK
status: open
blocked_by: []
claimed: true
---

## Question

Graduated from Not yet specified, found by the 071 audit. Since 071, the `Effect::Condition` arm in `bench-core/src/engine.rs` calls `break_concentration` when a condition incapacitates (Incapacitated, Paralyzed, Petrified, Stunned, Unconscious). A condition set with `bench override` skips that path, so a live agent who stuns a concentrating caster by override must also end the concentration by hand. Decide whether override should run the same check, and build it.

## Notes

Consult `tdd` and `code-review`. Look at how override writes a condition in `bench-core/src/live.rs`, and at `break_concentration` and the `incapacitates` set in `engine.rs`. If override can set conditions through more than one path (adding to a list, a whole-array write), cover each. Log the concentration end as the engine does. Update the live-mode reference if it says the agent ends concentration by hand. Bump the workspace version one patch, CHANGELOG under Changed. Batch numbers can't move (override is live only), so `POLICY_VERSION` stays 3 and the curve differs only in `engine_version` if regenerated.

## Resolution

<Filled in at close.>
