# Ticket 107 decision report: Build target teleport scoring

Run in auto mode. The ticket is a `task`, so no grilling took place. The implementer was the main thread and the auditor was a Haiku subagent. The auditor approved in round 2.

## Decisions

1. **Where the 0.25 lives.** The ticket asked for one named value shared by the push and the teleport. Options were a `PolicyConstants` field or a module constant. A module constant, `SHORT_DISPLACEMENT_SHARE`, was chosen, because a constants field would enter the policy fingerprint and mark the curve stale. Made by the implementer. Auditor: fine.
2. **How to share the melee-only check.** `forced_move_factor` built its attack list inline. The implementer pulled it into `melee_only(block)` for both callers, and the push score stays the same. Auditor: same behavior as before.
3. **What "holds no engagement" means.** The ticket defines it as no combatant listing the target and the target listing nobody, which matches how `band` reads both sides. The code checks both lists. A test covers an engagement that only one side lists. Made by the implementer. Auditor: fine.
4. **Old test `a_teleport_of_the_target_scores_nothing`.** Its claim is no longer true. The implementer replaced it with `a_self_teleport_scores_no_control`, which checks that Misty Step still scores survival and no control. Auditor: sound.
5. **Policy version.** The ticket says v4, but ticket 111 had already moved it to v5. No file under `data/` holds `"subject": "target"`, so the policy stays v5. Made by the implementer. Auditor: fine.
6. **Engine version and curve.** The workspace went from 0.1.32 to 0.1.33. `bench baseline generate` rewrote `data/baseline/curve.json`, and only `engine_version` changed. Auditor: confirmed the one-line diff.
7. **ADR 0012.** It already describes this build (ticket 097 wrote it), so it was left as is.
8. **Doc comments on new private items (audit round 1).** The auditor cited the user's rule to strip explanatory comments from internal functions. The surrounding file documents private items, but the user's rule takes precedence, so the comments were removed. Auditor: accepted in round 2.
9. **Vacuous zero checks (audit round 1).** The unengaged and ranged tests now assert `threat > 0.0` before checking that control is 0. Auditor: accepted.
10. **Cargo.lock (audit round 1).** The lockfile diff is only the two workspace version lines, and it goes in with the bump. Auditor: accepted.
11. **Changelog length (audit round 1).** The auditor asked for a shorter entry. The implementer kept it, because it sits under `## Unreleased` and follows the "The engine is 0.1.x" form of the entries below it. Auditor: accepted in round 2.

## Skipped gates

The runner's confirmation override skipped the human confirmation before implementation. The commit gate still applies.

## Checks

`cargo test --workspace` passes, including 478 bench-core lib tests. Clippy shows only the two warnings already present in `engine.rs`.
