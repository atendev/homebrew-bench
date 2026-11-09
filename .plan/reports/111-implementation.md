# 111 Build batch Counterspell and the Protective Magic split: decision report

Auto mode. Ticket type `task`, so no grilling subagent ran. One haiku auditor ran two rounds and approved on the second.

| Decision | Options | Choice | Made by | Audit |
| --- | --- | --- | --- | --- |
| The counter rule | Cast value against reserve plus cost as written; scale the value by the chance the save fails | As written in ADR 0019. A counter the caster is sure to save against is still priced at the full cast value | Main thread | Approved |
| What the rule reads for value | `Value::worth`; the plan scoring's caps | Damage capped at the target's hit points, healing capped at missing hit points, control as scored, and a heal on a downed target at twice its revived threat, as `heal_plan` scores a revival. `worth` leaves healing out | Main thread | Approved |
| Buffs | Score them; leave them at zero | Zero, because the ADR names damage, healing and control. Haste is never countered | Main thread | Approved |
| How the policy learns the targets | A field on the window; a transient field on `Reactions` | `target_ids` on `WindowTrigger::SpellCast`, defaulted so a prompt saved earlier loads | Main thread | Approved |
| Which reaction a counter uses | First offered; cheapest | Cheapest, the order `decide_hit` uses | Main thread | Approved |
| Usage key | Copy the recharge match; one helper | `usage_key`, used by `spend_usage` and `ability_ready` | Main thread | Approved |
| Mismatched `uses` across a group | Refuse at load; leave | Leave, as recharge groups are left | Main thread | Approved |
| Entry ids | Keep `protective-magic` for Shield; suffix both | `protective-magic-shield` and `protective-magic-counterspell`, group `protective-magic` | Main thread | Approved |
| The counter's DC | Set `dc` on each entry; add a `spellcasting` block | A block on the three fixtures, read from the Spellcasting action. They had none, so the DC stayed 0 and no counter could land. Only these three get it, so dragon casts don't change | Main thread | Approved |
| Attack bonus in that block | Leave out; DC minus 8 | DC minus 8, because the field is required and the SRD gives no value | Main thread | Approved |
| Importer with no Spellcasting action | Raise; record a blocking note | A class `b` note, which stops the monster graduating | Auditor, round 1 | Approved in round 2 |
| Reserve test | Keep; assert both sides of the reserve | Asserts the exact pass and take at hit points on each side of the computed reserve | Auditor, round 1 | Approved in round 2 |
| Slot pricing on the fixtures | Change the entries; no change | No change. The auditor asked, and the three fixtures have no `spell_slots`, so only the group pool is spent | Auditor raised, main thread answered | Approved in round 2 |
| Version | Patch; none | Workspace 0.1.30, policy v5 | Ticket notes | Approved |

## Curve

Only the Mage, Archmage and Lich moved. Wins are out of 100 fights in a cell.

| Monster | Cells moved | Win changes |
| --- | --- | --- |
| Mage | 15 | Level 2: 8 to 10. Level 3: 66 to 78. Level 4: 84 to 91 |
| Archmage | 20 | Level 3: 5 to 1. Level 4: 2 to 0. Level 5: 44 to 45. Level 6: 56 to 59. Level 7: 56 to 62. Level 8: 84 to 87. Level 9: 94 to 93. Level 11: 100 to 99 |
| Lich | 19 | Level 13: 1 to 0. Level 15: 9 to 6. Level 16: 16 to 12. Level 17: 69 to 51. Level 18: 74 to 52. Level 19: 92 to 79. Level 20: 96 to 90 |

The Mage got easier at levels 2 to 4. Fight logs show it spending reactions and uses from its 3/day pool on counters, and the default `resource_cost` is 0. The likely cause is less Shield, which I did not measure. The Lich has no limit and got harder. The rule follows the ADR, so this is recorded under Not yet specified on the map.

## Gates

Skipped gates: no requirements or implementation sign-off pause (runner override). The Jev audit asked for `to-spec` and `architect`, and neither ran because ADR 0019 and the ticket already specify the design. No remote CI runs on a push (`release.yml` runs on tags and pull requests), so step 6 is skipped.
