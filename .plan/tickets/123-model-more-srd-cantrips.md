---
id: 123
title: Model more SRD cantrips
type: task
mode: AFK
status: open
blocked_by: [114]
claimed: false
---

## Question

Model enough SRD cantrips in `data/spells/effects.json` that the cantrip rows of [Spell value metric](../closed/001-spell-value-metric.md) are not thin data.

## Notes

- Skills: `tdd`, `code-review`.
- Only three cantrips are modeled today (fire-bolt, resistance, spare-the-dying), the minimum for a percentile (ADR 0020). Dropping any one makes every cantrip row thin.
- Add combat cantrips through the scaffold (`scripts/scaffold-spells.py`) and hand-authored effects, as ADR 0014 sets out. Each gets `casting_time` and cantrip damage growth.
- A cantrip whose effect falls outside the vocabulary gets agent clauses or stays out, as other SRD spells do.
- `check-spell-effects.py` and the spell tests must pass. Changing the effects file changes the curve's fingerprint, so check the monster curve and regenerate only if cells move.

## Resolution

<!-- Filled in at close. -->
