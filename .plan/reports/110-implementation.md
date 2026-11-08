# 110 Build the spell-cast window and counter_spell effect: decision report

Auto mode. Ticket type `task`, so no grilling subagent ran. One haiku auditor ran two rounds and approved on the second.

| Decision | Options | Choice | Made by | Audit |
| --- | --- | --- | --- | --- |
| Where the window sits | Before `resolve_action` spends usage; inside `cast_spell` before the slot count | Inside `cast_spell`, before the slot count. The action and per-day usage are already spent, the slot is not, which is ADR 0019's cost of a countered cast | Main thread | Approved |
| Telling a reaction cast from a normal one | A parameter on every cast function; a flag on the request's reaction state | `Reactions.reacting`, set while `resolve_action` runs a reaction and restored on error | Main thread | Approved |
| How `counter_spell` carries the DC | Unit variant; struct with `dc` bound like a save | Struct with `dc` (0 takes the reactor's spell save DC), bound in `bind_effect` | Main thread | Approved |
| Reusing the save roll | Copy `resolve_save`'s head; extract a helper | Extracted `save_succeeds`, used by `resolve_save` and `counter_spell`. Roll order is unchanged and all old tests pass | Main thread | Approved |
| A counter save that waits on a player's die | Ignore; hold the cast | Hold the cast and discard the run, like a waiting window | Auditor, round 1 | Approved in round 2 |
| Batch policy for the new window | Price counters now; pass every window | Pass without a reason, so logs and numbers don't change until ticket 111 | Ticket notes | Approved |
| Counterspell in the effects file | Leave as agent clauses; add the effect | Added `counter_spell` and dropped the save and slot clauses. Fixtures keep the agent clause | Main thread | Approved |
| Version | Patch; none | Workspace 0.1.28, policy stays v4. The curve differs only in its engine version | Precedent from ticket 105 | Approved |
| Loader test | Rely on the shared check; add a test | Added one test for the spell-cast trigger on a reaction and on an action | Auditor, round 1 | Approved in round 2 |

Skipped gates: no requirements or implementation sign-off pause (runner override). The shell guard held one command that edited docs and the changelog, and the user chose to run it. No remote CI runs on push (`release.yml` runs only on tags), so step 6 is skipped. Thread rename was not attempted.
