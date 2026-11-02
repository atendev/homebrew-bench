# Does the standard party fall off at high levels

Research for ticket 052. Read on 2026-11-02 from the committed `data/baseline/curve.json` (policy v2, engine 0.1.0, 341 fixtures, 100 fights per cell, 20 round cap). `scripts/curve-tiers.py` regenerates the level, tier and effective CR tables below. The CR 19 and 20 fixture table comes from the same file.

## Short answer

Split. Levels 11 to 18 show no fall-off. Levels 19 and 20 read High, which is beyond what the SRD expects, but those two cells rest on four and three fixtures and the cause is not established.

Against on-CR fixtures the party wins 97.0%, 96.2% and 95.4% of fights in tiers 1 to 3, with PC deaths in about 5% of fights in each tier. Levels 11 to 16 match tiers 1 and 2. Levels 17 and 18 read Moderate, like most levels in every tier. Tier 4 as a whole falls to 80.1% wins and 19.9% deaths, and nearly all of that comes from levels 19 and 20 (64.5% and 67.3% wins). The SRD expects a single on-CR monster to stay a Low fight at every level, so those two cells are harder than the SRD expects.

The levels 19 and 20 shortfall comes mostly from two ancient dragons. The pit fiend also reads High there, narrowly (deaths in 15% and 17% of fights against a 15% Moderate limit). Dragons as a group read about a level harder than their CR in tiers 2 and 3 as well, while most non-dragons read at or below their CR from tier 3 up. The party gains against nearly every fixed high-CR monster from level 17 to 20, so it is not running out of power across the board.

Ticket 052's rule is: reopen magic items if the curve shows the party falling off at high levels, otherwise rule them out. The data does not clearly trigger it. The fall-off is two levels deep, rests on three or four fixtures, and is mostly one monster family that is also harder than its CR at lower tiers. A flat item bonus would raise every number without explaining that pattern. That last point is reasoning, not something this ticket tested. [Ticket 052](../../.plan/closed/052-does-the-standard-party-fall-off-at-high-levels.md) records the ruling: magic items are out of scope for this map, and the high-CR dragon and pit fiend gap becomes fog.

## What the SRD expects

SRD p. 202 says one monster of CR equal to party level is a Low fight for four characters. The XP budget table agrees across all 20 levels. A single on-CR monster's XP divided by the four-character Low budget is between 0.90 and 1.17 at every level, and between 0.95 and 1.00 at levels 11 to 20 (see `docs/research/009-cr-and-encounter-guidance.md`, sections 2 and 4). Nothing in the SRD predicts on-CR fights getting harder at high levels.

The Low, Moderate and High bands below are the Homebrew Bench thresholds from ADR 0011, not SRD numbers. Low means at least 95% wins and deaths in under 2% of fights. Moderate means at least 75% wins and deaths in under 15%.

## On-CR cells by party level

"Pooled CRs" follows the curve's rule: a CR with fewer than three fixtures pools the CRs one ladder step either side. "With left-out abilities" is the share of those fixtures fighting with at least one agent-clause or unmodeled ability ignored, which weakens the monster.

| Level | Fixtures | Pooled CRs | Win rate | Death rate | Band | Rounds median | With left-out abilities |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 27 | 1 | 99.6% | 2.9% | moderate | 2 | 48.1% |
| 2 | 44 | 2 | 96.3% | 5.0% | moderate | 4 | 47.7% |
| 3 | 27 | 3 | 96.4% | 7.8% | moderate | 4 | 44.4% |
| 4 | 20 | 4 | 95.9% | 4.5% | moderate | 5 | 65.0% |
| 5 | 27 | 5 | 99.6% | 0.9% | low | 4 | 74.1% |
| 6 | 11 | 6 | 95.3% | 6.6% | moderate | 4 | 54.5% |
| 7 | 6 | 7 | 99.3% | 0.8% | low | 5 | 50.0% |
| 8 | 10 | 8 | 96.5% | 3.9% | moderate | 5 | 60.0% |
| 9 | 8 | 9 | 93.0% | 7.2% | moderate | 5 | 62.5% |
| 10 | 6 | 10 | 83.2% | 17.0% | high | 6 | 33.3% |
| 11 | 7 | 11 | 99.7% | 0.3% | low | 5 | 85.7% |
| 12 | 17 | 11, 12, 13 | 93.9% | 6.1% | moderate | 5 | 82.4% |
| 13 | 8 | 13 | 94.8% | 5.2% | moderate | 4.5 | 87.5% |
| 14 | 3 | 14 | 97.3% | 2.7% | moderate | 5 | 100.0% |
| 15 | 4 | 15 | 95.0% | 5.2% | moderate | 4 | 100.0% |
| 16 | 5 | 16 | 94.8% | 5.2% | moderate | 4 | 40.0% |
| 17 | 4 | 17 | 88.8% | 11.2% | moderate | 4.5 | 75.0% |
| 18 | 5 | 17, 19 | 93.2% | 6.8% | moderate | 5 | 80.0% |
| 19 | 4 | 19, 20 | 64.5% | 35.5% | high | 6 | 100.0% |
| 20 | 3 | 20 | 67.3% | 32.7% | high | 7 | 100.0% |

No on-CR cell had a stalemate.

## By tier

| Tier | Levels | Fights | Win rate | Death rate | Levels reading Low | Rounds median range |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 1-4 | 11,800 | 97.0% | 5.1% | 0 of 4 | 2-5 |
| 2 | 5-10 | 6,800 | 96.2% | 4.4% | 2 of 6 | 4-6 |
| 3 | 11-16 | 4,400 | 95.4% | 4.6% | 1 of 6 | 4-5 |
| 4 | 17-20 | 1,600 | 80.1% | 19.9% | 0 of 4 | 4.5-7 |

On-CR fights read Moderate rather than Low at most levels in every tier. That is a steady offset between our thresholds and the SRD sentence, not a high-level effect. Rounds stay between 4 and 7 from level 2 up, so fights do not drag at high levels.

## Where the tier 4 drop comes from

The CR 19 and 20 fixtures at levels 17 to 20, as wins out of 100 fights:

| Fixture | CR | L17 | L18 | L19 | L20 |
| --- | --- | --- | --- | --- | --- |
| balor | 19 | 99 | 100 | 100 | 100 |
| pit-fiend | 20 | 51 | 60 | 85 | 83 |
| ancient-brass-dragon | 20 | 20 | 24 | 41 | 63 |
| ancient-white-dragon | 20 | 13 | 28 | 32 | 56 |

The level 20 cell is these three CR 20 fixtures. The pit fiend alone gives 83% wins at level 20, and the two dragons pull the cell down to 67%. The pit fiend is not a dragon and also reads High at levels 19 and 20, so dragons explain most of the drop but not all of it. All four get easier from level 17 to level 20, though the pit fiend dips from 85% to 83% between levels 19 and 20. Above CR 20 the party also gains against most fixtures, with three exceptions. The lich wins 6%, 2%, 5% and 11% for the party at levels 17 to 20, and the ancient gold dragon (CR 24) and the tarrasque (CR 30) give the party no wins at any of those levels.

Effective CR is the lowest party level where a fixture alone reads Low. The gap is effective CR minus CR, where positive means the monster is harder than its CR says. A fixture that never reads Low by level 20 counts as level 21, so its gap and any median it enters are lower bounds, marked "or more". "Dragon" means the fixture id contains "dragon", which includes dragon-turtle.

| Fixture CR | Group | Fixtures | Never Low by 20 | Median gap | Reads Low at or below its CR |
| --- | --- | --- | --- | --- | --- |
| 1-4 | all | 118 | 0 | +1 | 52 of 118 |
| 1-4 | dragon | 10 | 0 | +1 | 1 of 10 |
| 1-4 | non-dragon | 108 | 0 | +1 | 51 of 108 |
| 5-10 | all | 68 | 0 | +0 | 44 of 68 |
| 5-10 | dragon | 11 | 0 | +1 | 5 of 11 |
| 5-10 | non-dragon | 57 | 0 | +0 | 39 of 57 |
| 11-16 | all | 29 | 0 | +0 | 23 of 29 |
| 11-16 | dragon | 8 | 0 | +1 | 3 of 8 |
| 11-16 | non-dragon | 21 | 0 | -1 | 20 of 21 |
| 17-20 | all | 8 | 4 | +1 or more | 2 of 8 |
| 17-20 | dragon | 5 | 3 | +1 or more | 0 of 5 |
| 17-20 | non-dragon | 3 | 1 | -3 or more | 2 of 3 |

In tier 1, dragons and non-dragons both have a median gap of +1. From tier 2 up, the median dragon is a level or more harder than its CR, and no tier 4 dragon reads Low at or below its CR. Non-dragons get easier than their CR as CR rises. In tier 3, 20 of 21 read Low at or below their CR, and the iron golem (Low at level 17, CR 16) is the exception. In tier 4, balor and sphinx of valor read Low below their CR and the pit fiend never reads Low. Adult blue dragon is the clearest dragon exception and reads Low at level 13, three below its CR. The level 10 High cell comes from dragons too: young gold dragon and young red dragon win 53% and 54% for the party there, while aboleth, deva and stone golem win 100%.

One likely reason is that a dragon's breath weapon hits all four PCs at once. Policy v2 also takes no reactions and gives buffs no value (see the batch report method note), which may favor monsters that already deal heavy area damage. This ticket did not test either cause.

## Caveats

- Cells at levels 14 to 20 hold three to five fixtures, so one monster moves a cell by 20 to 30 points.
- Most high-CR fixtures fight with an ability left out (100% at levels 14, 15, 19 and 20). Left-out abilities weaken monsters, so the real high-level gap is probably wider than shown. That applies to dragons and non-dragons alike, so it does not change the split.
- The curve comes from one party, one tactics policy and one seed. Changing the party tables or the policy changes all of these numbers (ADR 0010, ADR 0011).

## What this means for the map

- Magic items stay out of the standard party, as [ADR 0010](../adr/0010-standard-party-of-four-generated-archetypes.md) decided. Levels 11 to 18 show no fall-off, and the levels 19 and 20 cells are too thin and too concentrated in a few monsters to justify a party-wide change. ADR 0010's consequence still holds: high-level numbers assume an item-free party and understate real parties.
- Dragons reading a level or more above their CR from tier 2 up, and the pit fiend and the lich at high CR, are new fog. This ticket did not settle whether that comes from the SRD's own CR choices, a policy v2 gap (no reactions, no spreading out against breath weapons), or an engine modeling gap.
