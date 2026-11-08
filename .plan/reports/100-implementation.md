# 100 Build darkness: decision report

Auto mode. Ticket type `task`, so no grilling subagent ran. One haiku auditor ran one round and approved.

| Decision | Options | Choice | Made by | Audit |
| --- | --- | --- | --- | --- |
| Where `darkness` sits in `unseen_reason` | Before Invisible; after Blinded | After Invisible and Blinded, so each event names one reason | Main thread | Approved |
| `meets` for `darkness` | Meets only its own key; also meets `dim_light` | Meets `darkness` and `dim_light`; `dim_light` does not meet `darkness` (ticket 091) | Main thread | Approved |
| Where the uneven-sight hint goes | New summary field; a warning | A warning in `summary.warnings`, so it reaches the JSON and the report with no schema change | Main thread | Approved |
| Hint replay command | Whole-batch command; a sample's replay | The first sample's replay with `--lighting darkness`, plus "drop `--fight` to rerun the batch". The batch command has no stored form without an index | Main thread | Approved, with a non-blocking note on the empty-samples fallback |
| Which combatants count for uneven sight | Whole party file; levels run | Monster plus party members at the levels run | Main thread | Approved |
| Curve | Leave stale; regenerate; edit the version line | Regenerated to a temp file and diffed: only `engine_version` changed (0.1.25 to 0.1.26), so I made that one-line edit. The embedded-curve CLI test fails on a stale engine version, and commit 4502693 set the precedent. Numbers under `unspecified` are unchanged | Main thread | Approved |
| Version | Patch; none | Workspace 0.1.26, `POLICY_VERSION` stays 4 | Ticket | Approved |
| Doc side effect | Leave; fix | ADR 0005 and the batch-mode reference said a `dim_light` key applies only under `dim_light` and that the engine has no darkness value. Both now say `darkness` meets it | Main thread | Not in the audited diff |

Skipped gates: no requirements or implementation sign-off pause (runner override). Thread rename succeeded. No remote CI on push (`release.yml` only), so step 6 is skipped. `cargo fmt --check` already differs on `bench/src/main.rs` imports before this change, and clippy warnings I saw are in untouched code.
