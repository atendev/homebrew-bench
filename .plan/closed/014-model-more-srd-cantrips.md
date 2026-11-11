---
id: 014
title: Model more SRD cantrips
type: task
mode: AFK
status: closed
blocked_by: []
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

Eight SRD combat cantrips join the effects file (engine 0.1.42): Acid Splash, Chill Touch, Poison Spray, Ray of Frost, Sacred Flame, Shocking Grasp, Starry Wisp and Vicious Mockery. With Fire Bolt, nine cantrips have damage the engine runs, so no cantrip row is thin. Damage dice grow through the engine's cantrip scaling at caster levels 5, 11 and 17.

- Acid Splash, Sacred Flame and Vicious Mockery deal no damage on a success. Acid Splash is a 5-foot Sphere at a chosen point.
- Ray of Frost lowers Speed by 10 feet until the end of the target's next turn, with a `notes` line on the gap from the SRD timing. `rounds: 1` would lapse when the caster's turn ends (see [Rounds per round for spell durations](../tickets/015-rounds-per-round-for-spell-durations.md)).
- The riders of Chill Touch, Shocking Grasp, Starry Wisp, Vicious Mockery and Sacred Flame's cover rule are agent clauses.
- Eldritch Blast (one attack roll per beam), Sorcerous Burst, Produce Flame, Shillelagh and True Strike stay out.
- `scripts/scaffold-spells.py` now defaults a damaging save without half-damage wording to `no_damage_on_success: true`. That surfaced Disintegrate, which deals full damage on a success, now its own ticket.
- No fixture casts the new cantrips and the curve does not fingerprint the effects file, so only the curve's engine version changed.

Decision report: [014-decisions](../reports/014-decisions.md).
