---
id: 064
title: Do dragons still read above their CR on the v3 curve
type: task
mode: AFK
status: open
blocked_by: [074]
claimed: false
---

## Question

On the baseline curve that ticket 074 regenerates for policy v3, do dragons still read a level or more above their CR from tier 2 up, and do the pit fiend and the lich still read hard at high CR? If they do, which cause fits the logs best: the SRD's own CR choices, area counts against breath weapons (the policy assumes a large area catches 4, so the party never spreads out), or an engine modeling gap?

## Notes

Ticket 052 found the gap (`docs/research/052-standard-party-at-high-levels.md`). Policy v3 adds the reaction half of the suspected policy gap ([Batch policy takes reactions](../closed/046-batch-policy-takes-reactions.md)). SRD dragons have no stat-block reactions, so for them v3 means Opportunity Attacks in both directions and the caster's Shield against their attacks. Both should make dragons read easier. Upcast scaling (ticket 059) makes the 14 dragons that cast "level N version" spells hit harder, which pushes the other way.

Method: compare effective CR to CR for the 43 dragon fixtures, the pit fiend and the lich on the old and new curves. Sample logs at the levels where they read High, and count how much party damage comes from breath weapons. Rerun a few cells with `policy.area_counts.large` at 2 and 3 as standing assumptions to see how much spreading out would move them. Write findings to `docs/research/064-dragons-on-the-v3-curve.md`.

If area counts explain most of the gap, open a grilling ticket on whether the policy should spread the party out against large areas. Otherwise close with the findings.

## Resolution
