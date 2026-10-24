---
id: 005
title: Survey prose-only monster abilities
type: research
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Which mechanics show up in the prose-only special abilities and actions in `data/srd/`, and how often?

## Notes

Group by mechanic (advantage under a condition, condition on failed save, regeneration, damage reduction, and so on) and count each group in both editions. Output a ranked list with 2 or 3 example monsters per group. This feeds the effects vocabulary in 006.

## Resolution

Surveyed the 2024 data only, since 002 made 2024 the baseline. Counts are uses across the 341 monsters. A name like Pack Tactics counts once per monster that has it.

### Groups, ranked by how much they matter to a fight sim

1. **Conditions applied by structured attacks.** 189 attacks and save actions have numbers in fields, but the condition they apply is only in the prose. Prone 59, Grappled 56, Restrained 37, Poisoned 24, Incapacitated 18, Blinded 16, Frightened 13, Paralyzed 12, others under 10. Examples: Wolf bite knocks Prone, Giant Octopus tentacle Grapples. This is the biggest gap.
2. **Area of effect.** 223 save actions. At least 79 name a Cone, Emanation, or Sphere in prose only (Lines exist too but use a format the count missed). How many party members a breath weapon hits depends on this. Examples: dragon breath, Fire Aura.
3. **Advantage or save bonuses under a condition.** Magic Resistance 34, Pack Tactics 20, Sunlight Sensitivity 5. Examples: Balor, Hell Hound, Kobold Warrior.
4. **Save overrides.** Legendary Resistance 34. Its uses per day are structured, the effect isn't.
5. **Legendary actions.** 32 monsters. Most are "make one X attack" or "move and attack" in prose (Pounce 18, Lash, Tail Swipe). The number of legendary actions per round (3, or 4 in lair) is missing from the data entirely.
6. **Reactions that change a hit.** Parry 7, Protective Magic 3, Riposte, Redirect Attack.
7. **HP triggers.** Regeneration 4, Undead Fortitude 2, Death Burst 5, Bloodied Fury or Frenzy 3.
8. **Start-of-turn damage near the monster.** Fire Aura 4, Stench 2, Mucus Cloud 1.
9. **Movement and escape.** Nimble Escape 6, Flyby 7, Leap 5, Teleport or Misty Step 7, Cunning Action 2, Spider Climb 16. These only matter if the engine tracks position.
10. **Spellcasting.** 60 actions. The spell list, DC, and per-day uses are structured. The spells themselves would need modeling.
11. **No combat effect.** Amphibious 31, Water Breathing 9, Hold Breath 6, Shape-Shift 25, the Restoration family 21 (returning after death), Illumination, Mimicry. Report as out of scope, don't model.

### What this means for the next tickets

- 004 has to decide whether the engine tracks position. Groups 2, 3 (Pack Tactics), 8, and 9 depend on it. A cheap middle ground is "engaged in melee or not" plus a target count for areas.
- 006 should start with condition riders, area target counts, advantage under a condition, Legendary Resistance, and legendary actions. Those five groups cover most monsters above CR 1.
- The importer or schema needs a field for legendary actions per round, since the SRD data leaves it out.
