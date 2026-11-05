# Do dragons still read above their CR on the v3 curve

Research for ticket 064. Read on 2026-11-04. It compares the v3 curve committed by ticket 074 (`data/baseline/curve.json`, policy v3, engine 0.1.15, 341 fixtures, 100 fights per cell, 20 round cap) with the v2 curve that ticket 052 read (policy v2, engine 0.1.0, from commit a035470's parent). `scripts/fixture-gaps.py OLD NEW` regenerates the effective CR tables. `scripts/area-count-sweep.py` regenerates the area-count and breath tables. Both scripts use ticket 052's definitions: effective CR is the lowest party level where a fixture alone reads Low (at least 95% wins and deaths in under 2% of fights), a fixture that never reads Low by level 20 counts as 21, the gap is effective CR minus CR, and "dragon" means the fixture id contains "dragon".

## Short answer

Mostly no. From tier 2 up the median dragon now reads at or below its CR: the median gap is +0 in tiers 2 and 4 and -2 in tier 3, down from +1 in all three. The ancient brass, white, black and copper dragons and the lich now read Low by level 19 or 20. The pit fiend reads Moderate at levels 19 and 20, with 96 and 97 wins and deaths in 4% and 3% of fights. That is one notch short of Low, and a resample with the batch command read Low at level 20, so its gap is within sampling noise.

In tier 2 only dragons moved: non-dragons stayed at +0. In tiers 1 and 3 the movement is party-wide. Tier 1 non-dragons moved from +1 to +0, tier 3 non-dragons from -1 to -2, and every on-CR cell from level 11 up now reads Low.

Seven dragons still read two or more levels above their CR: the green, silver and gold wyrmlings, the young white, young green and young silver dragons, and the dragon turtle. Breath weapons deal 72% to 80% of the damage the party takes in those fights. The test the ticket asked for, `policy.area_counts.large` at 3 or 2, moves one of the seven: the dragon turtle reaches its CR at either value. The other six breathe medium or small areas, so `large` doesn't touch them. Lowering the medium count to 2 as well brings the young white dragon from level 10 to level 7, one above its CR. The wyrmlings and the young green and silver dragons stay above their CR until every area hits a single target. For 30- and 60-foot cones the policy's counts already sit at or below the DMG's spread-out figure, so spreading the party further there would go past what the rules suggest. For the wyrmlings' 15-foot cones the DMG's spread-out figure is 1, and at 1 they still read a level above their CR.

The cause that fits best is the SRD's own CR choices for breath-heavy low and mid CR dragons. The engine's modeling gaps leave dragon abilities out, which makes dragons weaker, so they cannot make dragons read harder. Area counts do not explain most of the gap, so this ticket opens no grilling ticket on spreading the party out.

## Effective CR gap by tier

| Fixture CR | Group | Fixtures | v2 median gap | v3 median gap | v3: at or below CR |
| --- | --- | --- | --- | --- | --- |
| 1-4 | dragon | 10 | +1 | +1 | 1 of 10 |
| 1-4 | non-dragon | 108 | +1 | +0 | 71 of 108 |
| 5-10 | dragon | 11 | +1 | +0 | 6 of 11 |
| 5-10 | non-dragon | 57 | +0 | +0 | 49 of 57 |
| 11-16 | dragon | 8 | +1 | -2 | 8 of 8 |
| 11-16 | non-dragon | 21 | -1 | -2 | 21 of 21 |
| 17-20 | dragon | 5 | +1 or more | +0 | 4 of 5 |
| 17-20 | non-dragon | 3 | -3 or more | -4 or more | 2 of 3 |

"Or more" marks a median that includes a fixture never reading Low by level 20. Tier 1 is the one place dragons still read harder than non-dragons, and policy v3 did not move any wyrmling.

## Dragons that still read above their CR

The v3 gap of every dragon at +2 or more, with the v2 gap for comparison.

| Fixture | CR | v2 effective | v3 effective | v3 gap |
| --- | --- | --- | --- | --- |
| green-dragon-wyrmling | 2 | 4 | 4 | +2 |
| silver-dragon-wyrmling | 2 | 4 | 4 | +2 |
| gold-dragon-wyrmling | 3 | 5 | 5 | +2 |
| young-white-dragon | 6 | 10 | 9 | +3 |
| young-green-dragon | 8 | 11 | 10 | +2 |
| young-silver-dragon | 9 | 12 | 11 | +2 |
| dragon-turtle | 17 | 20 | 19 | +2 |

Every adult dragon now reads at or below its CR. The largest moves are adult copper (17 to 11), adult white (17 to 13) and adult gold (never Low to 17). The young gold and young red dragons, which made the v2 level 10 cell read High, moved from +3 and +4 to +1.

## CR 19 and up at levels 17 to 20

Wins and deaths out of 100 fights, v2 then v3.

| Fixture | CR | L17 | L18 | L19 | L20 |
| --- | --- | --- | --- | --- | --- |
| balor | 19 | 99w 1d to 100w 0d | 100w 0d to 100w 0d | 100w 0d to 100w 0d | 100w 0d to 100w 0d |
| ancient-brass-dragon | 20 | 20w 80d to 96w 4d | 24w 76d to 98w 2d | 41w 59d to 100w 0d | 63w 37d to 100w 0d |
| ancient-white-dragon | 20 | 13w 87d to 86w 14d | 28w 72d to 94w 6d | 32w 68d to 100w 0d | 56w 44d to 100w 0d |
| pit-fiend | 20 | 51w 49d to 80w 20d | 60w 40d to 86w 14d | 85w 15d to 96w 4d | 83w 17d to 97w 3d |
| ancient-black-dragon | 21 | 12w 88d to 91w 9d | 22w 78d to 94w 6d | 38w 62d to 98w 2d | 61w 39d to 100w 0d |
| ancient-copper-dragon | 21 | 7w 93d to 90w 10d | 10w 90d to 93w 7d | 19w 81d to 99w 1d | 27w 73d to 100w 0d |
| lich | 21 | 6w 94d to 88w 12d | 2w 98d to 90w 10d | 5w 95d to 98w 2d | 11w 89d to 99w 1d |
| solar | 21 | 23w 77d to 44w 56d | 28w 72d to 51w 49d | 49w 51d to 65w 35d | 69w 31d to 87w 13d |
| ancient-bronze-dragon | 22 | 1w 99d to 7w 93d | 2w 98d to 18w 82d | 16w 84d to 44w 56d | 19w 81d to 58w 42d |
| ancient-green-dragon | 22 | 0w 100d to 23w 77d | 0w 100d to 36w 64d | 2w 98d to 65w 35d | 4w 96d to 73w 27d |
| ancient-blue-dragon | 23 | 57w 43d to 88w 12d | 72w 28d to 91w 9d | 91w 9d to 100w 0d | 96w 4d to 100w 0d |
| ancient-silver-dragon | 23 | 1w 99d to 37w 63d | 2w 98d to 44w 56d | 11w 89d to 66w 34d | 28w 72d to 89w 11d |
| kraken | 23 | 99w 1d to 100w 0d | 99w 1d to 100w 0d | 100w 0d to 100w 0d | 100w 0d to 100w 0d |
| ancient-gold-dragon | 24 | 0w 100d to 2w 98d | 0w 100d to 3w 97d | 0w 100d to 10w 90d | 0w 100d to 22w 78d |
| ancient-red-dragon | 24 | 1w 99d to 6w 94d | 3w 97d to 6w 94d | 4w 96d to 24w 76d | 9w 91d to 15w 85d |
| tarrasque | 30 | 0w 100d to 0w 100d | 0w 100d to 0w 100d | 0w 100d to 0w 100d | 0w 100d to 0w 100d |

Fixtures above CR 20 are beyond the party's levels, so the curve can't say whether they read above their CR. The lich reads Low at level 20, below its CR of 21, so it no longer reads hard.

## Breath weapons and area counts

The batch policy assumes an area catches a fixed number of opponents by size class: 2 for small, 3 for medium and 4 for large (`policy.area_counts`). It hits that many PCs, so the party never spreads out. Wyrmling breaths (15-foot cones) read as small, young dragon breaths (30-foot cones) as medium, and adult and ancient breaths as large.

The sweep reran each fixture over levels 1 to 20 with five settings: default, `large=3` and `large=2` (the ticket's test), `medium=2` with `large=2`, and every area at 1. It reads breath share from all 100 fight logs at one level. Breath share is the share of all damage dealt to PCs that came from an action whose id contains "breath".

| Fixture | CR | default | large 3 | large 2 | medium and large 2 | all 1 |
| --- | --- | --- | --- | --- | --- | --- |
| green-dragon-wyrmling | 2 | 4 | 4 | 4 | 4 | 3 |
| silver-dragon-wyrmling | 2 | 4 | 4 | 4 | 4 | 3 |
| gold-dragon-wyrmling | 3 | 5 | 5 | 5 | 5 | 4 |
| young-white-dragon | 6 | 10 | 10 | 10 | 7 | 5 |
| young-green-dragon | 8 | 9 | 9 | 9 | 9 | 7 |
| young-silver-dragon | 9 | 11 | 11 | 11 | 11 | 9 |
| adult-white-dragon | 13 | 14 | 13 | 11 | 11 | 10 |
| adult-green-dragon | 15 | 15 | 13 | 12 | 12 | 11 |
| dragon-turtle | 17 | 19 | 17 | 17 | 17 | 17 |
| ancient-white-dragon | 20 | 19 | 19 | 17 | 17 | 16 |
| pit-fiend | 20 | 20 | 20 | 20 | 20 | 20 |
| ancient-gold-dragon | 24 | never | never | never | never | never |

| Fixture | Level | default | large 3 | large 2 | medium and large 2 | all 1 |
| --- | --- | --- | --- | --- | --- | --- |
| green-dragon-wyrmling | 3 | 98w 2d, breath 80% | 98w 2d, breath 80% | 98w 2d, breath 80% | 98w 2d, breath 80% | 100w 1d, breath 64% |
| silver-dragon-wyrmling | 3 | 97w 4d, breath 78% | 97w 4d, breath 78% | 97w 4d, breath 78% | 97w 4d, breath 78% | 100w 1d, breath 57% |
| gold-dragon-wyrmling | 4 | 98w 2d, breath 80% | 98w 2d, breath 80% | 98w 2d, breath 80% | 98w 2d, breath 80% | 100w 0d, breath 71% |
| young-white-dragon | 7 | 82w 19d, breath 78% | 82w 19d, breath 78% | 82w 19d, breath 78% | 99w 1d, breath 76% | 100w 0d, breath 47% |
| young-green-dragon | 9 | 99w 1d, breath 77% | 99w 1d, breath 77% | 99w 1d, breath 77% | 100w 0d, breath 66% | 100w 0d, breath 0% |
| young-silver-dragon | 10 | 68w 32d, breath 77% | 68w 32d, breath 77% | 68w 32d, breath 77% | 89w 11d, breath 72% | 100w 0d, breath 59% |
| adult-white-dragon | 13 | 98w 2d, breath 81% | 100w 0d, breath 77% | 100w 0d, breath 74% | 100w 0d, breath 74% | 100w 0d, breath 48% |
| adult-green-dragon | 15 | 100w 0d, breath 77% | 100w 0d, breath 76% | 100w 0d, breath 65% | 100w 0d, breath 67% | 100w 0d, breath 0% |
| dragon-turtle | 18 | 97w 3d, breath 72% | 100w 0d, breath 63% | 100w 0d, breath 41% | 100w 0d, breath 41% | 100w 0d, breath 0% |
| ancient-white-dragon | 18 | 87w 13d, breath 66% | 98w 2d, breath 60% | 100w 0d, breath 55% | 100w 0d, breath 55% | 100w 0d, breath 0% |
| pit-fiend | 20 | 99w 1d, breath 0% | 99w 1d, breath 0% | 99w 1d, breath 0% | 99w 1d, breath 0% | 99w 1d, breath 0% |
| ancient-gold-dragon | 20 | 15w 85d, breath 60% | 33w 67d, breath 48% | 62w 38d, breath 39% | 62w 38d, breath 39% | 71w 29d, breath 0% |

With every area at 1, a single-target breath often scores below the dragon's Multiattack, so some dragons stop breathing and the share drops to 0%.

What the sweep shows:

- Breath is most of the damage in every dragon cell, between 60% and 81% under the default counts.
- `large=3` and `large=2` change only large areas. Both bring the dragon turtle from 19 to its CR of 17, and adult white and green one to three levels lower. Of the seven outliers, only the dragon turtle breathes a large area.
- Adding `medium=2` moves the young white dragon three levels, to one above its CR. It does not move young green or young silver, whose wins rise at the sampled level but whose effective CR stays put. It cannot move wyrmlings, whose small areas already count 2.
- With every area at 1, the wyrmlings read one level above their CR, young silver reads at its CR and young green one below. That would mean every breath catches one PC.
- The pit fiend has no area attack, so area counts don't explain it.

## What the rules say about area counts

The 2024 DMG's Targets in Area of Effect table (DMG chapter 3, "Mobs", Adjudicating Areas of Effect, p. 86-87) puts 2 creatures in a 15- or 20-foot cone, 5 in a 30-foot cone and 16 in a 60-foot cone. It subtracts 1d3 when the targets are spread out. Against a party of four:

- Wyrmling 15-foot cone: 2 by the table. Spread out, 2 minus 1d3 leaves 1 at most, since the breath always catches the creature it is aimed at. The policy uses 2.
- Young dragon 30-foot cone: 5 by the table, so all 4 PCs, and 3 on average if spread out. The policy uses 3.
- Adult and ancient 60-foot cones: 16 by the table, all 4 PCs even when spread out. The policy uses 4.

For 30- and 60-foot cones the policy's counts are already at the DMG's spread-out figure or more generous to the party. The settings that closed gap there, `medium=2` for young dragons and `large` at 3 or 2 for the dragon turtle, assume a party more spread out than the DMG suggests. For 15-foot cones the policy's 2 is the table's unadjusted figure, above the spread-out figure of 1. The all-1 setting matches the spread-out figure, and the three wyrmlings still read a level above their CR there. The DMG does not say what area counts the SRD's CR choices assumed, so this ticket can't test that directly.

## Which cause fits

- **Area counts.** The ticket's `large` test explains the dragon turtle. Lowering the medium count too explains most of the young white dragon. Neither explains the wyrmlings, young green or young silver within the DMG's own numbers. Apart from the wyrmlings' spread-out figure of 1, which leaves them a level above their CR, the settings that help assume a party spread wider than the DMG table does.
- **Engine modeling gaps.** These push the other way. `bench fixture show <id>` lists each ability's `kind`, and batch skips every `unmodeled` ability by default (`default_resolution: skipped`, reported as a "skipped" finding). The curve's `ignored_abilities` lists only agent clauses, so it doesn't show these. Every adult dragon has at least two unmodeled abilities: Pounce on all but the blue dragon, and Spellcasting on all but the white dragon. The young silver dragon's Paralyzing Breath is unmodeled. The dragon turtle fights with the agent clauses of its Bite and Tail ignored, and the pit fiend with its Bite's. Left-out abilities weaken a monster, so they can't make it read harder.
- **The SRD's own CR choices.** These fit the rest best. The outliers are breath-heavy dragons at low and mid CR, such as the young silver dragon's 11d8 Cold Breath at DC 17 at CR 9 or the young white dragon's 9d8 at DC 15 at CR 6. They read hard at area counts the DMG would accept, and the young silver dragon does so with its second breath left out. This ticket did not separate that from the standard party's Constitution saves, which every outlier breath targets.

## Caveats

- `scripts/area-count-sweep.py` runs `bench batch`, which seeds cells differently from `bench baseline generate`. Its default column therefore differs from the curve by sampling noise, up to about 8 wins in a cell (young silver dragon at level 10: 68 wins in the sweep, 76 on the curve). The young white dragon's default effective CR is 10 in the sweep and 9 on the curve, and the pit fiend's is 20 in the sweep and never Low on the curve.
- Effective CR depends on the Low band's 2% death limit, so one PC death in 100 fights moves a fixture by a level.
- The curve still comes from one party, one tactics policy and one seed (ADR 0010, ADR 0011).

## What this means for the map

- The dragon gap that ticket 052 found has mostly closed under policy v3, as has the high-CR gap for the pit fiend and the lich.
- Area counts do not explain most of what is left, so no grilling ticket opens on spreading the party out against large areas.
- The remaining outliers are seven low and mid CR breath-heavy dragons. Batch reports for homebrew built like them will read above their claimed CR, and the cause is the monster's breath weapon damage at its CR, not a policy or engine gap.
