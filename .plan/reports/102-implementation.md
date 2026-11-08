# 102 Hit-triggered Reflexive Antennae and Protective Magic: decision report

Auto mode. Ticket type `task`, so no grilling subagent ran. One haiku auditor ran three rounds and approved on the third.

| Decision | Options | Choice | Made by | Audit |
| --- | --- | --- | --- | --- |
| Where the import change lives | `corrections.json`; a prose pattern in the importer | Prose patterns beside Parry. Corrections can't set a trigger and record upstream errors | Main thread | Approved |
| How Protective Magic casts Shield | A `cast` effect on the reaction; a cast step through the Spellcasting action | A `cast` effect with no slot level. `cast_spell` reads the effects file and never a Spellcasting list, so no engine change was needed | Main thread | Approved |
| Counterspell clause | Agent clause on the entry; drop it | An agent clause, as the ticket says. The engine logs it after each Shield | Ticket notes | Approved |
| `default_resolution` on Protective Magic | Drop it and refuse batch until assumed; keep `skipped` | Drop it, as Riposte does. A kept `skipped` would report a modeled Shield as not modeled. `bench batch` on the Mage, Archmage and Lich now needs `--assume abilities.protective-magic=ignored` | Main thread | Accepted, flagged to the user |
| Reflexive Antennae's kind | `modeled`, as the ticket's shape implies; `partly_modeled` with a clause | `partly_modeled` with an agent clause saying Antennae is not modeled. The action it uses has no effects, so `modeled` was false | Auditor, round 1 | Approved in round 2 |
| `default_resolution` on Reflexive Antennae | Drop it; keep `skipped` | Keep `skipped`, so rust monster batches run as before and report a skipped mechanic. The importer's `lit` flag became `keep_default` | Main thread | Approved in round 2 |
| Reflexive Antennae pattern | Any "uses X" sentence; only "uses Antennae" | Only "uses Antennae", because the clause describes that action | Main thread | Approved in round 2 |
| ADR 0018 consequence line | Delete it; scope it to the original imports | Scope it in the past tense and add ticket 102's effect. The auditor asked for deletion, but the old line is true of the original imports | Auditor raised it, main thread chose the fix | Approved in round 3 |
| Version | Patch; none | Workspace 0.1.29, policy stays v4 | Ticket notes | Approved |

Curve: the Mage moves in 15 cells, the Archmage in 19 and the Lich in 19. The Mage gets harder, for example 66 party wins of 100 at level 3, down from 85. The Rust Monster moves in no cell, because Antennae has no modeled effect. The ticket expected all four fixtures to move.

Skipped gates: no requirements or implementation sign-off pause (runner override). No remote CI runs on push (`release.yml` runs only on tags), so the CI step does not apply.
