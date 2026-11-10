# Ticket 112 decision report: Build Near after a teleport

Run in auto mode. The ticket is a `task`, so no grilling took place. The main thread implemented it, and a Haiku subagent audited it. The auditor approved in round 1.

## Decisions

1. **Setting name.** The ticket asked for a recorded setting like `settings.reaction_windows` but gave no name. The implementer chose `teleport_near`, a field on `EncounterSettings` and `StartSpec` with `serde(default, skip_serializing_if = Not::not)`. Old saves load it as false and re-serialize unchanged. `bench start` and batch set it to true. Auditor: fine.
2. **Which entries change.** The options were to overwrite only entries that exist, or to insert Near where an entry is missing. Ticket 108 keeps the missing-entry case out of scope, so the code changes existing entries only, on each side separately. The test `a_gated_teleport_adds_no_missing_range_entry` checks this. Made by the implementer. Auditor: matches the ticket.
3. **Pair set.** The code collects the freed pairs before the engagements clear. The set is the subject's own list plus every combatant whose list holds the subject. The self teleport test makes both links one-sided, so each half of the union is covered. A duplicate in the union writes Near twice, which does no harm. Made by the implementer. Auditor: correct. It noted that one test covers the "other list holds the subject" path through the fighter only, and called that non-blocking.
4. **Ungated behavior.** With the setting off, the old code path runs unchanged. `an_ungated_teleport_leaves_every_band_as_it_was` compares every `ranges` map before and after. Auditor: fine.
5. **Curve and policy version.** Ticket 108 said to bump the policy only if curve cells moved. The implementer bumped the engine from 0.1.33 to 0.1.34 and reran `bench baseline generate` at the default 100 fights and seed. `data/baseline/curve.json` changed only in `engine_version`, so the policy stays v5. Auditor: the diff agrees.
6. **ADR 0005.** The teleport sentence now names the gate and the Near write on both sides. It says Near is a modeling choice, keeps "bands unchanged" for legacy saves, notes the missing-entry case, and says the legacy branch stays until a saves-migration ticket drops it. Auditor: covers every point in the ticket.
7. **ADR 0012.** The price stays `threat(target) × 0.25`. Only the wording changed: the teleport now leaves freed pairs at Near, the 0.25 is a constant, and the sentence that said ticket 108 might change the price now says it kept it. Auditor: fine.
8. **Changelog.** One entry under Unreleased, in the "The engine is 0.1.x" form of the entries below it. Made by the implementer.
9. **Comments.** No explanatory comments in internal code. The new pub fields carry doc comments, which the user's rule allows. Auditor: confirmed.
10. **Line length (audit note).** Two new lines in `engine.rs` run past 120 characters. The surrounding code has lines that long, so they stay. Auditor: style only.
11. **Jev skills.** The audit recommended `/to-spec` and `/architect`. The ticket was already a full spec and the change touched one code block, so neither was run. Made by the main thread.

## Skipped gates

The runner's confirmation override skipped the human confirmation before implementation. The commit gate still applies.

## Checks

`cargo test --workspace` passes, including 483 bench-core lib tests. The two new gated tests failed with the Near write disabled and pass with it enabled. Clippy shows only the two warnings that were already in `engine.rs`.
