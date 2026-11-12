# Ticket 013 decisions: spell lift runner

Branch `ticket-013-spell-lift-runner`. Engine 0.1.44. Audit: haiku, one round, APPROVE with two fixes applied (member copies, stalemate wording) and no re-check needed.

| Decision | Options | Choice | Made by | Audit |
|---|---|---|---|---|
| Fight count | Fewer for speed; the panel's 100 | 100 per member in every row, so a six-member row pools 600 paired fights. `--fights` overrides | Main thread | Approved |
| Interval method | Bootstrap; Wilson on rates; normal interval on paired differences | Mean ± 1.96 standard errors of the per-fight paired differences, over every panel fight in the row. One sentence: spells are level with each other when their 95% intervals overlap. Recorded in ADR 0020 | Main thread | Approved |
| What lift is | Difference of rates; mean of paired fight differences | Mean of paired differences for win and for "a party member died". Signed. A round-cap stalemate counts as not won | Main thread; auditor raised the stalemate wording | Approved after fix |
| Row control win rate | Hide it; report it | Each row reports its pooled control win rate and whether it is in the 40 to 80% band, so cantrip and upcast rows show when their control left the band | Main thread | Approved |
| Swap | Add the spell; replace the level's spells | The control drops level-L spells. The spell run also drops any ability that already casts the tested spell, then adds one in the section its `casting_time` names, cast at the row's slot level (`upcast: true` above its level; cantrips at-will, no slot) | Main thread | Approved |
| Reaction trigger | Refuse reaction spells; guess a trigger | A counter spell triggers on an enemy cast; any other reaction triggers when the caster is hit by an attack roll. The row carries a note saying so | Main thread | Approved |
| Not measured | Run anyway; mark | `long` spell; an SRD spell's upcast row when it has an upcast clause; a counter spell where the panel has no spellcasting fixture. A homebrew upcast clause runs as its standing assumption says, with a note | Main thread | Approved |
| Never cast | Report zero; report apart | `never_cast` when the party casts the spell in no fight of the row, with no lift numbers | Main thread | Approved |
| Per-cast diagnostic | Engine event change; count in the batch log | `FightResult.party_casts` tallies casts per spell and the damage and healing raised by the caster's own events before the next cast. Later-round damage is not credited, so ADR 0020 calls it a diagnostic. Heal amounts are parsed from the event message to avoid changing the event schema | Main thread | Approved |
| Control reuse | Cache controls across spells; rerun per spell | Rerun. A single spell never repeats a control, and `member_fights` is public so the baseline ticket can cache. A test shows two spells get identical control fights | Main thread | Approved |
| Seeds | `--seed` honoured; fixed | Fixed from the panel (ADR 0007). `--seed` and `--levels` are refused with `--test-spell` | Main thread | Approved |
| Sample and replay | Reuse monster-batch replay; own command | One sample per measured row: the first fight the control lost and the spell won. Replay is `--test-spell <id> --row <party level> --member <fixture>:<single\|group> --fight <index>`. A party level names one row of a spell, so `--row` is enough | Main thread | Approved |
| Output | Print only; also write | JSON on stdout and `lift.json` plus `logs/` under `reports/<id>-lift/`. The HTML report belongs to the spell lift report ticket | Main thread | Approved |
| Threads | Use all cores | `--threads`, default 4, per AGENTS.md | Main thread | Approved |
| Engine version | Hold; bump | 0.1.44. Curve and panel regenerated with `--threads 4`; they differ from before only in `engine_version` | Main thread | Approved |

Not done here: the batch table of SRD lift numbers (spell baseline), the report page, and the skill. Gates skipped under the runner's confirmation override: the main thread wrote requirements and implementation notes itself. Remote CI: none runs on branch pushes (`release.yml` runs on pull requests and version tags), so the CI step does not apply. Thread renamed to "Spell lift runner". `AGENTS.md` still has your uncommitted CPU-limit edit and `.cargo/` is untracked; I left both out of the commit.
