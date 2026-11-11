# Ticket 012 decisions: control party and spell panel

Branch `ticket-012-control-party-and-spell-panel`. Engine 0.1.43. Audit: haiku, two rounds, APPROVE.

| Decision | Options | Choice | Made by | Audit |
|---|---|---|---|---|
| What the control removes | Abilities whose cast uses a slot of level L; abilities that cast a spell whose own level is L | The spell's own level, read from the SRD spell book. The whole ability goes, in any section, so Shield leaves the reactions list in a level 1 control. Slots and every other ability stay. A cast of a spell the book lacks stays, because its level is unknown | Main thread | Approved |
| Which members change | Caster role only; any member that casts | Caster role only, found by `party::role_rank`, as ADR 0020 puts every tested spell on the caster | Main thread | Approved |
| Party levels with a panel | 1 to 20; the odd levels 1 to 17 | Odd levels 1 to 17. Every headline, upcast and cantrip row in ADR 0020 sits at 2N-1 for N from 1 to 9 | Main thread | Approved |
| Which control the band is measured with | The headline control, without level (P+1)/2; one panel per row's control | The headline control. ADR 0020 asks for one fixed panel per party level. A row for another spell level at the same party level, such as a cantrip row, fights the same fixtures against its own control, and the band holds only for the headline control. The ADR now says both | Main thread; auditor raised the cantrip case | Approved on re-check |
| Band edges | Exclusive; inclusive | Inclusive, 0.40 to 0.80, with a 1e-9 tolerance on win rates rounded to four places | Main thread | Approved |
| Fixtures per shape | All in band; a fixed cap | Up to 3 per shape, nearest 60% first, then by id. Every spell baseline run fights each member, so a short panel keeps that cost down | Main thread | Approved |
| Spellcasting fixture | Any curve spellcaster; an in-band spellcaster | In band only, since every panel member must be. When none of the three picks casts, the best-placed in-band caster replaces the last pick. With none in band the level records `spellcasting_fixture: null`. Levels 1 and 5 have none | Main thread; auditor asked which reading | Approved on re-check |
| What counts as a spellcaster | Has a `spellcasting` block; casts a spell the SRD file models | Casts a spell the SRD file models, since a counter spell needs a cast to stop. Dragons count | Main thread | Approved |
| Group shape | Several copies of any fixture; copies of a lower-CR fixture | 4 copies, one per party member, of a fixture the control beats more than 80% of the time alone, with a CR below every single-shape fixture's. The CR rule came from the audit. The committed panel met it already, so its members and fingerprint did not change | Main thread; auditor raised the CR rule | Approved on re-check |
| Fielding several monsters | New fight loop in the panel module; `monster_copies` on `BatchSpec` | `monster_copies` on `BatchSpec`. `bench batch` always sets 1 and live start already numbers duplicates `ogre-1`, `ogre-2` | Main thread | Approved |
| Paired seeds | Store a base seed only; store each member's master seed | Each member stores its master seed, derived from the base seed, its id and its shape. A spell run with that seed draws the same fight seeds as the control, and a test checks it | Main thread | Approved |
| Fingerprint contents | The whole file; the selection only | The selection: party level, control spell level, shape, id, copies and seed. Remeasured win rates leave it alone. The file also records input fingerprints and an effects-file hash, and `stale_reasons` checks all three | Main thread | Approved |
| Shared code | Copy the worker pool and stale check; extract them | Extract `run_parallel` and `stale_reasons` from `baseline.rs`. The regenerated curve differs only in its engine version, so the refactor moved no numbers | Main thread | Approved |
| CLI | Library only; `bench baseline panel` plus status | `bench baseline panel [--fights] [--seed] [--threads]`, and `bench baseline status` gains `spell_panel` | Main thread | Approved |
| Committed data | Generate later; commit now | Commit `data/baseline/spell-panel.json` now, generated with `--threads 4`. A calibration test holds it to its inputs, the band and the CR rule, and one member regenerates to its committed numbers | Main thread; auditor checked it ships with the code | Approved |
| Engine version | Hold; bump | 0.1.43 with the curve regenerated, as each earlier build ticket did | Main thread | Approved |

Not done here: the lift runner (ticket 013) builds each row's own control and decides whether to report a row's control win rate when it falls outside the band. `AGENTS.md` has your uncommitted CPU-limit edit, which names `bench baseline generate` but not `bench baseline panel`. I left that file alone.

Gates skipped under the runner's confirmation override: the main thread wrote the requirements and implementation notes itself, without a subagent. No remote CI runs on branch pushes. `release.yml` runs only on pull requests and version tags, and this repo merges locally, so the CI step does not apply.
