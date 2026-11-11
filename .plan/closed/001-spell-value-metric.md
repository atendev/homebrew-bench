---
id: 001
title: Spell value metric
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

What number does the batch report compare a homebrew spell with the SRD spells of its level on? Candidates:

- Expected damage or healing per cast, at each slot level
- Expected damage or healing per slot spent, so a 1st-level and a 3rd-level spell can be read on one scale
- Change in win rate for a fixed fight with the spell added to a caster's list

The answer decides what the baseline is measured in, and whether upcast and cantrip scaling count toward it.

## Notes

- Grill with `grilling` and `domain-modeling`.
- Read the Upcast and Cast level entries in `CONTEXT.md`, and [ADR 0014](../../docs/adr/0014-spell-effects-file-authorship.md) for the scaling rules.
- Settled earlier: the comparison is against SRD spells of the same level, not monster CR.

## Resolution

Spell lift carries the verdict. It is the change in the standard party's win rate and casualty rate when the caster role's spells of the tested level are swapped for the tested spell, against a control run with that level's spells removed. Expected damage or healing per cast is reported beside it, ungraded. Win rate is the only candidate that puts Fireball, Hold Person and Haste on one scale, and it reuses the monster baseline's engine, party and policy. Recorded in [ADR 0020](../../docs/adr/0020-spell-lift-as-the-spell-value-metric.md). CONTEXT.md gains Spell lift, Spell panel and Spell baseline.

Upcast and cantrip scaling count. Each gets its own row, not one folded number:

- The headline row is party level 2L-1, cast at level L.
- Each upcast level N gets a row at party level 2N-1, but only for a spell with an upcast rule. An upcast clause is marked not measured.
- Cantrips get rows at levels 1, 5, 11 and 17. Only three cantrips are modeled today, the minimum for a percentile, so dropping any one makes those rows thin data. More should be added.

Decided in auto mode by the grilling subagent, which took every recommendation:

1. Verdict is spell lift. Per-cast damage or healing is a diagnostic only.
2. The report gives placement among same-level SRD spells with named neighbours, and no pass/fail. "Reads like level N" appears only outside the level's range. A row with fewer than three SRD spells is thin data and gets no percentile.
3. Each party level has a fixed, fingerprinted spell panel of fixtures at about 40 to 80% control win rate, measured in the control run. It has single-monster and group shapes. Placement uses the mean lift, and both columns are shown. Upcast rows use their own level's panel.
4. Swap, not add. Every tested spell goes on the caster role. A spell the policy never casts in a row reads "never cast", not zero.
5. Rows as listed above.
6. Party side only in batch, and the report says so.
7. The SRD numbers are a committed spell baseline under `data/baseline/`. It records fingerprints of the effects file, party, panel, policy, engine and fight count, and stale reports are flagged.
8. Paired seeds and a fixed fight count per panel fight. The report gives an interval, and overlapping neighbours are "level with" the homebrew.
9. The panel includes a spellcasting fixture where the curve has one. Otherwise counter spells read "not measured" for that row.
10. An entry that can't be modeled is refused at intake, as the map settled. A modeled spell with no combat effect the engine runs reads "not measured", never zero. Whether intake also refuses that kind stays with Homebrew spell intake rules.
11. The rest of the caster's list stays, so concentration competition counts.
12. Glossary terms: Spell lift, Spell panel, Spell baseline.
