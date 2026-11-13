# Ticket 019 decisions: Disintegrate damage on a successful save

Branch `ticket-019-disintegrate-save`. Engine 0.1.45. Audit: haiku, two rounds, APPROVE.

| Decision | Options | Choice | Made by | Audit |
|---|---|---|---|---|
| The fix | Set `no_damage_on_success: true`; `half_on_success: true`; change the engine's default for a save with neither flag | `no_damage_on_success: true`. The SRD gives damage only on a failed save. The engine default stays, since Disintegrate was the only entry written that way | Main thread | Approved |
| Test | Disintegrate only; an invariant over every damaging save in the effects file | Both. One test checks Disintegrate's flags. The other walks the embedded JSON, including saves nested in areas and riders, and fails on any damaging save that sets neither flag, so a later entry can't repeat the bug. Both failed before the data change and pass after it | Main thread | Approved |
| Engine version | Hold; bump | 0.1.45. The effects file is embedded in the binary, and each earlier data ticket bumped the version | Main thread | Approved |
| Curve | Regenerate only if cells move; version only | Regenerate. The version bump makes the curve stale anyway, and cells moved. At levels 11 to 20 only, 202 cells change their win count (193 down, by up to 43 of 100). Levels 1 to 10 are unchanged | Main thread | Approved |
| Spell panel | Regenerate; leave stale | Regenerate. The panel hashes the effects file, so the calibration test fails without it. Members change at party levels 13, 15 and 17, where the control party keeps Disintegrate; levels 1 to 11 are unchanged | Main thread | Approved |
| Threads | Default cores; `--threads 4` | `--threads 4` under `nice -n 19`, per AGENTS.md. The curve took about 6 minutes and the panel about 6 | Main thread | Approved |
| Why the falls are biggest against dragons | Blame Legendary Resistance; state the numbers only | Numbers only. The first changelog draft blamed Legendary Resistance. The auditor asked for a check, and the fixtures hold it as `unmodeled` with a `skipped` default, so the engine never applies it. The cause is not established | Auditor raised it, main thread fixed | Approved on re-check |
| What the commit stages | Everything modified; this ticket's files | This ticket's files. `AGENTS.md` and `.cargo/` were the user's uncommitted changes before the session and stay out | Auditor raised it, main thread answered | Approved on re-check |

Not investigated: why dragon cells fall the most.

Under the runner's confirmation override, the main thread wrote the implementation notes itself, without a subagent, and skipped that gate. No remote CI runs on branch pushes. `release.yml` runs only on version tags, so the CI step does not apply.
