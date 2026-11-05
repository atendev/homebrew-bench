---
id: 081
title: Override ends concentration on incapacitating conditions
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Graduated from Not yet specified, found by the 071 audit. Since 071, the `Effect::Condition` arm in `bench-core/src/engine.rs` calls `break_concentration` when a condition incapacitates (Incapacitated, Paralyzed, Petrified, Stunned, Unconscious). A condition set with `bench override` skips that path, so a live agent who stuns a concentrating caster by override must also end the concentration by hand. Decide whether override should run the same check, and build it.

## Notes

Consult `tdd` and `code-review`. Look at how override writes a condition in `bench-core/src/live.rs`, and at `break_concentration` and the `incapacitates` set in `engine.rs`. If override can set conditions through more than one path (adding to a list, a whole-array write), cover each. Log the concentration end as the engine does. Update the live-mode reference if it says the agent ends concentration by hand. Bump the workspace version one patch, CHANGELOG under Changed. Batch numbers can't move (override is live only), so `POLICY_VERSION` stays 3 and the curve differs only in `engine_version` if regenerated.

## Resolution

Built. `bench override` snapshots which concentrating creatures are free of incapacitating conditions, applies the override, and for each that is now incapacitated (Incapacitated, Paralyzed, Petrified, Stunned, Unconscious, the `STOPS_ACTING` set) calls `engine::end_concentration`, a new public wrapper over `break_concentration`. It ends what the spell held on others, fires `on_end` (Haste lethargy) and logs `<id> loses concentration` after the override line. The RNG stream is read from and written back to `state.rng`, so replay stays deterministic. An override that leaves the conditions alone, or sets a non-incapacitating one, doesn't touch concentration.

Engine 0.1.18, `POLICY_VERSION` stays 3. The regenerated `data/baseline/curve.json` (1m44s) differs only in `engine_version`. Test: `override_ending_concentration_with_an_incapacitating_condition` (stunned ends it and removes a held condition on another creature, prone and an hp override leave it). Workspace tests pass. CHANGELOG under Changed, and the live-mode reference now says so and that overriding `<id>.concentration` clears only the field.

Audit: haiku, two rounds. Round 1 found the live-mode sentence pointed agents at `<id>.concentration` as if it ended linked effects, and the test didn't cover held conditions; both fixed, round 2 approved. Left open as fog: overriding `end_state` to Unconscious or Dead, or `concentration` onto an already-incapacitated creature, doesn't end concentration. No CI runs on branch pushes (`release.yml` runs on pull requests and tags only), so nothing was pushed.
