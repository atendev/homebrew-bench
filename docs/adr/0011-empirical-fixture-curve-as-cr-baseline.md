# Empirical fixture curve as the CR baseline

SRD 5.2 has no per-CR stat benchmarks, so the report cannot cite a table for expected hit points, damage or armor class. We build our own yardstick instead: every fixture is fought against the standard party at levels 1 to 20 under the batch tactics policy, and the committed results form the baseline curve a homebrew monster is compared against. The SRD's one calibration sentence (a single monster of CR equal to party level is a Low-difficulty fight for four characters, SRD p. 202) anchors what the curve means.

## Considered options

- SRD prose anchor only: coarse, one binary verdict point, and the win/casualty thresholds would still be ours to invent.
- Empirical fixture curve: chosen. Same engine, same party, same tactics, so the comparison is apples-to-apples, and it is license-clean as our own derived analysis of CC-BY-4.0 stat blocks.
- User-supplied 2024 DMG numbers: unconfirmed, paywalled, and not shippable under the SRD license. Left open as a possible user-local file after phase 1.

## Consequences

- Win rate and casualty profile carry the verdict because the SRD difficulty definitions are stated in wins and casualties. The Low/Moderate/High thresholds are our own calibration (Low: at least 95% wins, deaths in under 2% of fights; Moderate: at least 75% wins, deaths in under 15%) and must be labeled as ours, never as SRD numbers.
- Mismatches are stated as direction plus effective CR, the lowest party level where the monster reads Low, clamped to "around 1 or trivial" below and "over 20" above. No pass/fail, and no advice wording.
- Rounds and damage per round never grade against fixed thresholds. They flag anomalies against the fixture spread only.
- The curve goes stale when fixtures, party tables, the tactics policy or the engine change. The baseline lives as a committed table under `data/baseline/`, regenerated deliberately, with input fingerprints recorded; a report computed against a stale baseline says so. The report footer carries the SRD 5.2.1 attribution and says the curve is derived from SRD stat blocks.
- Multi-monster homebrew does not fit the single-monster curve and is fog for a later ticket.
