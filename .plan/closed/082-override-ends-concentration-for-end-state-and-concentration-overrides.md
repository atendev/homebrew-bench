---
id: 082
title: Override ends concentration for end state and concentration overrides
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Graduated from Not yet specified, found by the 081 audit. `bench override` now ends concentration when it leaves a concentrating creature with an incapacitating condition. Two paths still skip it: overriding `end_state` to Unconscious or Dead, and overriding `concentration` onto a creature that is already incapacitated (or dead or unconscious). Decide whether override should run the same cleanup for those, and build it.

## Notes

Consult `tdd` and `code-review`. Look at the override code in `bench-core/src/live.rs` (the 081 snapshot-and-end logic), `engine::end_concentration`, and how the engine ends concentration when a creature drops to 0 HP or dies. A creature that is Unconscious or Dead can't concentrate, so the likely answer is yes. Cover each override path. Log the concentration end as 081 does. Update the live-mode reference and the CHANGELOG under Changed, and bump the workspace version one patch. Batch numbers can't move (override is live only), so `POLICY_VERSION` stays 3 and the curve differs only in `engine_version` if regenerated.

## Resolution

Built. After `bench override` applies, any creature that concentrates and can't (its `end_state` is Unconscious, Stable or Dead, or it holds an incapacitating condition) loses concentration through `engine::end_concentration`, which ends what the spell held on others, fires `on_end` and logs `<id> loses concentration`. The check compares each combatant before and after the override and acts on one that was able before, whose concentration changed, or that is new, so it covers `<id>.end_state`, `<id>.concentration` onto an incapacitated or down creature, and the `combatants.<id>` and `combatants.<index>` path forms, and an override elsewhere leaves a stale creature alone. New `engine::cannot_concentrate`; `is_incapacitated_by_condition` is private again.

Engine 0.1.19, `POLICY_VERSION` stays 3. The regenerated `data/baseline/curve.json` (2m00s) differs only in `engine_version`. Test: `override_ending_concentration_for_end_state_and_concentration_overrides`. Workspace tests pass. CHANGELOG under Changed plus a Replay compatibility bullet, and the live-mode reference now lists the covered paths.

Audit: haiku, four rounds (the user extended the two-round limit). Round 1: global scan risked replay divergence, test gaps. Round 2: the first fix missed `combatants.<id>` paths (a regression from 0.1.18). Round 3: `is_none_or` broke the declared MSRV 1.80 and the replay note was missing. Round 4 approved. Two items the audit raised were deferred as fog: removal (`bench remove`, clearing `combatants.<id>`) leaves what a concentrator held, and a pre-0.1.18 save's stale concentration on a creature an override targets with the same value. No CI runs on branch pushes (`release.yml` runs on pull requests and tags only), so nothing was pushed.
