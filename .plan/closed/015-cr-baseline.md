---
id: 015
title: CR baseline
type: grilling
mode: HITL
status: closed
blocked_by: [009, 014]
claimed: false
---

## Question

What are monster results compared against? Which CR guidance numbers does the report use, what win rate and round count counts as "on CR" for a given party level, and how does the report state a mismatch?

## Notes

Use the CR and encounter guidance findings and the standard party decision. Licensing limits found in the research bound what the repo can ship. Consult `grilling` and `domain-modeling`.

## Resolution

Resolved 2026-10-28 by grilling. See `docs/adr/0011-empirical-fixture-curve-as-cr-baseline.md`.

- The yardstick is an empirical curve: every fixture fought against the standard party at levels 1 to 20 under the batch tactics policy, anchored to the SRD p. 202 calibration line that gives the curve its meaning. The report also states cited SRD facts as a factual layer (the monster's XP and its budget-table difficulty at each party level). DMG numbers stay out of the repo; a user-local file may come after phase 1.
- The baseline encounter shape is one monster against the four-member standard party. Multi-monster homebrew is fog for a later ticket.
- Win rate and casualty profile carry the verdict, matching the SRD difficulty definitions. Difficulty bands are our own calibration, committed as a designer-set table: Low is at least 95% party wins with deaths in under 2% of fights, Moderate is at least 75% wins with deaths in under 15%, High is anything riskier. A casualty means a death, not a downed PC. These numbers are labeled as ours, not SRD numbers.
- Bands are computed per (CR, party level) cell from the fixture distribution. A CR with too few fixtures pools plus or minus one CR; a cell still thin is labeled "thin data" instead of being smoothed silently.
- Effective CR is the lowest party level where the monster reads Low, in whole levels, clamped to "around 1 or trivial" below range and "over 20" above.
- Rounds and damage per round never grade against fixed thresholds. They flag anomalies against the fixture spread only, such as fights running long for the claimed CR.
- Each (monster, party level) cell runs 100 fights, designer-tunable. The seeds decision makes every cell reproducible.
- A mismatch is stated with four parts, in order: the verdict at the claimed CR against the fixture band, in both directions including "no threat at this level"; the effective CR; anomalies with thin-data cells named; and a one-sentence basis line. Neutral observation only, no pass/fail, no advice wording.
- The curve ships as a committed table under `data/baseline/`, regenerated deliberately, with fingerprints of its inputs (fixture set, party tables, tactics version, engine version). A report against a stale baseline says so. The report footer carries the SRD 5.2.1 attribution and says the curve is derived from SRD stat blocks.

Surfaced: multi-monster comparison and user-local DMG numbers as fog. Unblocks Report format.
