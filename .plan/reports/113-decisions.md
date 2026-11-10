# Ticket 113 decisions

1. **Which ticket.** The only frontier ticket was 113, a task. Main thread. No audit needed.
2. **Setting name and plumbing.** The ticket named `settings.long_durations`. It is a field on `EncounterSettings` and `StartSpec`, with `serde(default, skip_serializing_if = "Not::not")`. `bench start` and batch set it true. Saves without it load false. Main thread, following ticket 112. Auditor: pass.
3. **Where the round is stored.** `started_round: u32` on `ActiveModifier` and `ConditionInstance`, skipped at 0. Set from `state.round` where `Effect::Condition` and `Effect::TimedModifier` apply. Main thread. Auditor: pass.
4. **When long durations expire.** Options: check only in `finish_turn`, or also when the round counter advances. Choice: both. `decrement_durations` calls the new `expire_long_durations`, and `start_next_turn` calls it right after `round += 1`, firing `on_end` as `finish_turn` does. With the check only in `finish_turn`, a minute would clear a turn late. Main thread. Auditor: timing and `on_end` correct.
5. **Gate-off path.** The `Rounds` arms are untouched. The new `Minutes | Hours | Days` arms are guarded `if long` and sit before the old arms, which stay as they were. Main thread. Auditor: pass.
6. **Test arithmetic.** A minute applied in round 1 clears when round 11 starts, after 10 full rounds. Main thread. Auditor: pass.
7. **Existing See Invisibility test.** It asserted the permanent workaround. I turned the gate on in it and renamed it, since an hour is 600 rounds. Main thread.
8. **Curve and policy.** I regenerated the curve with the gate on. Only `engine_version` changed, so policy stays v5. Main thread.
9. **Version.** Engine 0.1.35, with a changelog entry and an ADR 0005 paragraph that states the legacy branch's removal plan. Main thread.
10. **Audit findings.** Minor: doc comments on two private functions (stripped, per the user's rule) and a missing blank line (fixed). Not acted on: a replay assertion for old saves, because the `old_state()` check matches ticket 112's `teleport_near` test, and the event wording for modifier expiry, because it matches the `Rounds` path's string.
11. **Skipped gates.** No requirements or implementation confirmation was asked, per the runner's override. The thread rename was not tried.
