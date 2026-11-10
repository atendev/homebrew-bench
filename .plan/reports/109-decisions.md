# Ticket 109 decision report: Durations longer than a round

Run in auto mode. The ticket is a grilling ticket, so the main thread wrote six questions with options and a recommendation each, and one Sonnet subagent answered them in a single round. A Haiku subagent audited the result and approved in round 1.

## Decisions

1. Does `rounds` belong in this ticket? Options: fix it too, leave it per-turn and record it as fog, or decide the direction now and build later. Choice: leave it per-turn. It moves Shield and Haste play and batch cells, so it needs its own curve regeneration. The main thread recommended it and the grilling subagent accepted. Auditor: matches the resolution and the new fog item.
2. How do minutes, hours and days count? Options: convert to rounds against the round clock, never expire inside an encounter, or keep the per-turn decrement. Choice: convert, at 10, 600 and 14400 rounds. A one-minute Blinded then clears after 10 rounds, and an hour or a day outlasts any fight. Recommended by the main thread, accepted by the grilling subagent. Auditor: fine.
3. How is the length stored? Options: a `started_round` field on `ActiveModifier` and `ConditionInstance`, rewriting units into rounds on apply, or a per-round counter. Choice: the `started_round` field, so players and agents still see `hours: 1`. The subagent added the edge that expiry is checked as the round counter advances and in `finish_turn`, and the build ticket now says so. Auditor: consistent with the code at `engine.rs` 1985 and `state.round`.
4. How do old saves refold? Options: a recorded `settings.long_durations` gate, or no gate. Choice: the gate, as `reaction_windows` and `teleport_near` do. Old saves keep the per-turn decrement. Recommended by the main thread, accepted by the subagent. Auditor: fine.
5. See Invisibility. Choice: restore `{"hours": 1}` and drop its agent clause in the build. It scores 0, so cells should not move, and the build must confirm the Charm Person and Sunburst cells first. Accepted by the subagent, who added the confirmation step.
6. Follow-up tickets. Choice: one build ticket, 113, for decisions 2 to 5, with `rounds` as its own fog item. Accepted by the subagent, who also added three acceptance checks, which the ticket carries.
7. Ticket layout. The repo's `.plan/` is flat, so the map and tickets sit at `.plan/map.md` and `.plan/tickets/`, not under `maps/<slug>/`. The repo README wins over the global layout note. Made by the main thread.
8. Audit note. The auditor asked that ticket 113 name ADR 0005 as the owner of the duration vocabulary. The main thread applied it.
9. Jev skills. The audit recommended `/to-spec` and `/architect`. This is a decision ticket with no code, so neither was run. Made by the main thread.

## Skipped gates

The runner's confirmation override skipped the human confirmation before and after the decision. The commit gate still applies. The audit's model mismatch halt was released by the user switching to Sonnet 5.5 and replying "proceed".

## Checks

No code changed, so no tests ran. The auditor read the diff and the cited code and found every link, file name and line reference correct.
