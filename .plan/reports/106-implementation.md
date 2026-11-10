# 106 Build plan-failure warnings by role: decision report

Auto mode. Ticket type `task`, so no grilling subagent ran. One haiku auditor approved in the first round.

| Decision | Options | Choice | Made by | Audit |
| --- | --- | --- | --- | --- |
| Role match | Copy the `group_party` match; one helper | `party::role_rank`, used by `group_party` and `failure_labels` | Main thread | Approved |
| Where roles resolve | In `plan_failure_warnings`; in `Log::plan_failed` | `Log::plan_failed`, since the combatant list is there. `PlanFailure` gains `role_keyed` | Main thread | Approved |
| Role uniqueness | Count the level's party; count party-side combatants in the fight | Party-side combatants in the fight, which is the level's party in batch | Main thread | Approved, with a note on summons |
| Id rewrite | Parse ids from text; resolve through combatants | Combatants, longest id first, whole tokens, no rescan | Ticket notes | Approved |
| Log lines | Rewrite; leave | Leave the raw engine error | Ticket notes | Approved |
| Row key | Label only; label and `role_keyed` | Both, so a monster named like a role never merges with a role row | Main thread | Approved |
| CHANGELOG heading | Added; Changed | Added, like the 070 warning entry | Main thread | Style note only |
| Cleanup of the master worktree | Remove; leave | Left in /tmp, since the auto-mode classifier denied the removal | Main thread | Not audited |
