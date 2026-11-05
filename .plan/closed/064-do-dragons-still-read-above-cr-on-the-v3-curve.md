---
id: 064
title: Do dragons still read above their CR on the v3 curve
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

On the baseline curve that ticket 074 regenerates for policy v3, do dragons still read a level or more above their CR from tier 2 up, and do the pit fiend and the lich still read hard at high CR? If they do, which cause fits the logs best: the SRD's own CR choices, area counts against breath weapons (the policy assumes a large area catches 4, so the party never spreads out), or an engine modeling gap?

## Notes

Ticket 052 found the gap (`docs/research/052-standard-party-at-high-levels.md`). Policy v3 adds the reaction half of the suspected policy gap ([Batch policy takes reactions](../closed/046-batch-policy-takes-reactions.md)). SRD dragons have no stat-block reactions, so for them v3 means Opportunity Attacks in both directions and the caster's Shield against their attacks. Both should make dragons read easier. Upcast scaling (ticket 059) makes the 14 dragons that cast "level N version" spells hit harder, which pushes the other way.

Method: compare effective CR to CR for the 43 dragon fixtures, the pit fiend and the lich on the old and new curves. Sample logs at the levels where they read High, and count how much party damage comes from breath weapons. Rerun a few cells with `policy.area_counts.large` at 2 and 3 as standing assumptions to see how much spreading out would move them. Write findings to `docs/research/064-dragons-on-the-v3-curve.md`.

If area counts explain most of the gap, open a grilling ticket on whether the policy should spread the party out against large areas. Otherwise close with the findings.

## Resolution

Mostly no. Findings are in `docs/research/064-dragons-on-the-v3-curve.md`. `scripts/fixture-gaps.py` and `scripts/area-count-sweep.py` regenerate its tables.

- From tier 2 up the median dragon now reads at or below its CR. The median gap fell from +1 to +0 in tier 2, from +1 to -2 in tier 3 and from +1 or more to +0 in tier 4, and every adult dragon reads at or below its CR. Tier 1 dragons stay at +1, against +0 for non-dragons.
- The pit fiend reads Moderate at levels 19 and 20 (96 and 97 wins, deaths in 4% and 3% of fights). A resample reads Low at 20, so its gap is sampling noise. The lich reads Low at level 20, below its CR of 21.
- In tier 2 only dragons moved. In tiers 1 and 3 the shift is party-wide: non-dragons moved from +1 to +0 and from -1 to -2, and every on-CR cell from level 11 up reads Low.
- Seven dragons still read +2 or more: the green, silver and gold wyrmlings, the young white, green and silver dragons, and the dragon turtle. Breath deals 72% to 80% of party damage in their cells.
- The ticket's test, `policy.area_counts.large` at 3 or 2, brings the dragon turtle to its CR and moves none of the other six, which breathe medium or small areas. Lowering `medium` to 2 as well brings the young white dragon to +1. The wyrmlings and the young green and silver dragons need every area to hit one PC. For 30- and 60-foot cones the policy's counts (3 and 4) are already at or below the 2024 DMG's Targets in Area of Effect figure for spread-out targets. For 15-foot cones the policy's 2 is above the spread-out figure of 1, but at 1 the wyrmlings still read a level above their CR.
- Engine gaps push the other way, since skipped unmodeled abilities (Pounce, Spellcasting, the young silver's Paralyzing Breath, listed by `bench fixture show`) weaken dragons. The best fit for what remains is the SRD's own CR choices for breath-heavy low and mid CR dragons. This ticket did not separate that from the standard party's Constitution saves.

Area counts don't explain most of the gap, so no grilling ticket opens on spreading the party out.
