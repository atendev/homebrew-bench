---
id: 082
title: Override ends concentration for end state and concentration overrides
type: task
mode: AFK
status: open
blocked_by: []
claimed: true
---

## Question

Graduated from Not yet specified, found by the 081 audit. `bench override` now ends concentration when it leaves a concentrating creature with an incapacitating condition. Two paths still skip it: overriding `end_state` to Unconscious or Dead, and overriding `concentration` onto a creature that is already incapacitated (or dead or unconscious). Decide whether override should run the same cleanup for those, and build it.

## Notes

Consult `tdd` and `code-review`. Look at the override code in `bench-core/src/live.rs` (the 081 snapshot-and-end logic), `engine::end_concentration`, and how the engine ends concentration when a creature drops to 0 HP or dies. A creature that is Unconscious or Dead can't concentrate, so the likely answer is yes. Cover each override path. Log the concentration end as 081 does. Update the live-mode reference and the CHANGELOG under Changed, and bump the workspace version one patch. Batch numbers can't move (override is live only), so `POLICY_VERSION` stays 3 and the curve differs only in `engine_version` if regenerated.

## Resolution

<Filled in at close.>
