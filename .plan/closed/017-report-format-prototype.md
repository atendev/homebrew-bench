---
id: 017
title: Report format
type: prototype
mode: HITL
status: closed
blocked_by: [011, 014, 015]
claimed: false
---

## Question

What does the batch report contain and how is it laid out? It should cover win rate, rounds and damage per round across party levels, the comparison with CR guidance, flagged wording problems, standing assumptions, ignored abilities, and links to a few sampled logs.

## Notes

Make a mock report from invented numbers for the user to react to, using the `prototype` skill. Link the mock as an asset. The log format, standard party and CR baseline decisions set what the report can say.

## Resolution

Resolved 2026-10-28. The prototype is a static, accessible HTML report at [`docs/prototypes/report-format.html`](../../docs/prototypes/report-format.html), using invented Ashen Maw results. It establishes the proposed reading order:

1. Prototype warning and report identity.
2. Headline claimed CR, effective CR, directional comparison, difficulty band, and caveats.
3. Level results with wins, deaths, win rate, difficulty band, median and mean rounds/DPR, P10-P90 spread, and thin-data flags.
4. Project-derived CR interpretation beside a separate SRD reference block.
5. Wording and coverage findings with modeled, agent-clause, inert, skipped, and default resolutions.
6. Standing assumptions.
7. Reproducibility metadata and three deterministic sampled-log links.
8. Method, limitations, and the exact SRD 5.2.1 attribution.

The mock keeps the result qualified when skipped mechanics may affect the outcome, distinguishes project calibration from SRD facts, and labels every value as invented. It uses no live data, filters, or runtime dependency. The linked text logs are in `docs/prototypes/logs/`.
