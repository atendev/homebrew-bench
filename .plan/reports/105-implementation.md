# 105 Build See Invisible: decision report

Auto mode. Ticket type `task`, so no grilling subagent ran. One haiku auditor ran two rounds and approved on the second.

| Decision | Options | Choice | Made by | Audit |
| --- | --- | --- | --- | --- |
| See Invisibility's duration | Keep `{"hours": 1}` as the ticket says; fix the tick in this ticket; store `permanent` | `permanent`, with an agent clause giving the 1-hour length. A test showed `{"hours": 1}` ends with the caster's own turn. Fixing the tick changes Charm Person and Sunburst and the replay hashes of saves holding such a duration | Main thread | Objected in round 1 because no decision was recorded. Approved in round 2 once the ticket, the map and ticket 109 recorded it |
| The tick itself | Fix now; open a ticket | New grilling ticket, 109 Durations longer than a round | Main thread | Approved |
| Which ability lists carry the trait | Traits only; any list | Any list, through a shared `StatBlock::abilities`, because the ticket says "like `no_provoke`" | Main thread | Approved |
| Test after the duration change | Keep the name; rename | Renamed to say the spell is held for the encounter. It still fails if the entry goes back to `{"hours": 1}`. No test covers a 1-hour length, which the auditor noted as non-blocking | Main thread, on the auditor's objection | Approved |
| Live-mode line on who can see a mover | Leave it; add See Invisible | Added. The ticket named only the other sentence, and this one would have been wrong | Main thread | Approved |
| ADR 0005 additive-variant list | Leave it; add `see_invisible` | Added | Main thread | Approved |
| Curve | Leave stale; edit the version line after a regeneration | Regenerated to a temporary file and diffed. Only `engine_version` changed, so I edited that line | Main thread | Approved |
| Version | Patch; none | Workspace 0.1.27, `POLICY_VERSION` stays 4 | Precedent from ticket 100 | Approved |

Skipped gates: no requirements or implementation sign-off pause (runner override). The shell guard held one command that edited `data/spells/effects.json`, and the user chose to run it. Thread rename succeeded. No remote CI runs on push (`release.yml` runs only on tags), so step 6 is skipped.
