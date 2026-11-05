---
id: 075
title: Should Haste's batch value count concentration risk and short fights
type: grilling
mode: HITL
status: open
blocked_by: []
claimed: false
---

## Question

Under policy v3 the standard caster casts Haste on the striker on its first turn from level 5, and that makes the party weaker in every cell ticket 056 measured. Should the buff value count the chance the caster's concentration breaks inside the horizon (lethargy then costs the bearer a turn), the chance the fight ends before the horizon, or both, and does that change ship under v3 before the baseline regeneration or as a later v4?

## Notes

Consult `grilling` and `domain-modeling`. The formula is in ADR 0012 and [Build v3 scoring for buffs and forced movement](../closed/056-build-v3-scoring-for-buffs-forced-movement-and-teleports.md); 045 and 055 chose not to score lethargy.

Evidence from 056 after its audit fixes (100 fights per cell, seed 7, master against the branch): against the adult red dragon the party's wins drop from 100 to 92 at level 17 and stay level at level 11 (34 to 35). Over those 100 fights per level the caster casts Haste 200 and 140 times, Haste ends 168 and 122 times, and 95 and 69 extra attacks land. Against the ogre and the knight every fight is still won, but rounds and monster DPR rise (knight level 17: rounds 1.07 to 1.41, DPR 3.86 to 5.39), because most fights end in one or two rounds.

Possible inputs: the caster's chance to keep concentration against the incoming damage it expects (a Constitution save against DC max(10, half the damage)), the expected rounds left from the opponents' remaining HP against the party's damage, and lethargy as a lost bearer turn. If the answer changes scoring under v3, [Regenerate the baseline for policy v3](074-regenerate-the-baseline-for-policy-v3.md) should wait for its build.

## Resolution
