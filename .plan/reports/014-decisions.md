# Ticket 014 decisions: model more SRD cantrips

Branch `ticket-014-model-more-srd-cantrips`. Engine 0.1.42. Audit: haiku, two rounds, APPROVE.

| Decision | Options | Choice | Made by | Audit |
|---|---|---|---|---|
| Which cantrips to add | Every SRD cantrip; combat cantrips the vocabulary runs; a minimal three more | Eight combat cantrips: Acid Splash, Chill Touch, Poison Spray, Ray of Frost, Sacred Flame, Shocking Grasp, Starry Wisp, Vicious Mockery. With Fire Bolt that makes nine with damage the engine runs, so no cantrip row is thin | Main thread | Approved |
| Eldritch Blast | Model as one attack whose dice scale; agent clause; leave out | Leave out. The engine scales a cantrip by multiplying dice on one roll, while Eldritch Blast makes a separate attack per beam and can split them across targets | Main thread | Approved |
| Sorcerous Burst, Produce Flame, Shillelagh, True Strike | Model with placeholders; leave out | Leave out. They need a damage type choice and exploding dice, a bonus-action summon then a Magic action, or a held weapon | Main thread | Approved |
| Save success on Acid Splash, Sacred Flame, Vicious Mockery | Scaffold default (full damage on a success); `no_damage_on_success: true` | No damage on a success. With both flags false the engine deals full damage on a success | Main thread | Approved |
| Acid Splash shape | Single-target save, as Fireball is written; 5-foot Sphere at a chosen point | 5-foot Sphere at `target_point`, as Ice Storm and Sleep are written | Main thread | Approved |
| Ray of Frost duration | `rounds: 1`; `until_end_of_next_turn` on the target; agent clause | `until_end_of_next_turn`. `rounds` ticks at every turn end, so `rounds: 1` would lapse when the caster's turn ends, and its fix is the open "Rounds per round" ticket. The target's next turn is the only turn of the target inside the SRD window. A `notes` line records the gap | Main thread | Approved |
| Shocking Grasp rider | `reactions` modifier with `rounds: 1`; agent clause | Agent clause. The modifier blocks every reaction, not only Opportunity Attacks, and its duration hits the same `rounds` problem | Main thread | Approved |
| Other riders (Chill Touch, Starry Wisp, Vicious Mockery, Sacred Flame cover) | Timed modifiers; agent clauses | Agent clauses, following Guiding Bolt and Faerie Fire for "next attack roll" and Invisible riders | Main thread | Approved |
| Range on save cantrips | Add a range; leave it out | Leave it out. `SavingThrow` has no range field and no SRD save entry records one | Auditor raised it, main thread answered | Accepted on re-check |
| Poison Spray | Con save at 10 feet (2014); ranged attack at 30 feet (SRD 5.2.1) | Ranged attack at 30 feet, from the SRD 5.2.1 text | Auditor raised it, main thread answered | Withdrawn after a rules lookup |
| Scaling test coverage | Levels 4 and 17; every breakpoint | Levels 4, 5, 10, 11 and 17 for all eight | Auditor raised it, main thread fixed | Approved on re-check |
| Scaffold default | Leave it; default a damaging save without half-damage wording to no damage on a success | Change the default. A fresh scaffold gave these three cantrips full damage on a success | Main thread | Approved |
| Disintegrate | Fix it here; new ticket | New ticket "Disintegrate damage on a successful save", blocking "Spell baseline". The standard caster knows Disintegrate from level 11, so the fix can move curve cells. That is a separate change from adding cantrips | Main thread | Approved |
| Monster curve | Regenerate; bump engine version only | Engine version only. No fixture casts the new cantrips, the curve does not fingerprint the effects file, and the calibration tests pass | Main thread | Approved |
| Engine version | Hold; bump | 0.1.42, as each earlier build ticket bumped it | Main thread | Approved |

Left out: Disintegrate still deals full damage on a successful save until its ticket lands.

Gates skipped under the runner's confirmation override: the main thread wrote the requirements and implementation notes itself, without a subagent. No remote CI runs on branch pushes. `release.yml` runs only on version tags, so the CI step does not apply.
