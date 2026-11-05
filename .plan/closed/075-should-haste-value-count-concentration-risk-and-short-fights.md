---
id: 075
title: Should Haste's batch value count concentration risk and short fights
type: grilling
mode: HITL
status: closed
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

Decided by an agent under the user's delegation, from the code (`tactics.rs` at 3f89ef6), ADR 0012, tickets 045, 046, 055 and 056, and seed-fixed release batches of a scratch prototype built outside the repo.

The buff value counts all three: the caster's chance to keep concentration, the rounds the fight has left, and lethargy as a lost bearer turn. It ships under policy v3 before the regeneration, in build ticket [076](../tickets/076-build-haste-value-for-concentration-risk-and-short-fights.md), so ticket 074 still regenerates once.

1. **Rounds left.** `rounds_left(actor) = Σ hit_points(o) / Σ damage_threat(a)` over the actor's active opponents o and its allies a (the actor included) that can act. `damage_threat` is `threat` without the healing term, so a healer's Cure Wounds doesn't shorten the estimate. When the allies' damage is 0, the estimate is `BUFF_HORIZON`. A modifier's horizon becomes `h = min(horizon(duration), rounds_left)` and may be fractional. Reason: against the knight at level 17 the party's damage clears its HP in about half a round, and an extra attack that starts next turn is worth little there.
2. **Keep chance.** `k`, the chance the caster's concentration survives one round of opponent turns, counts only for `concentration` modifiers whose source is the actor. Other modifiers use `k = 1`. Each opponent that can act contributes its best ready action against the caster, the action with the most attack-roll plus saving-throw damage (`roll_value` with `Rolls::Attacks(Any)` and `Rolls::Saves(None)`). Its attack damage counts only when the caster is that opponent's top-priority target (`ranked_opponents(foe, None)[0]`, the same pick `extra_action_value` makes). Its save damage always counts, because areas catch the caster without targeting it. Then:
   - When the counted damage meets the caster's `hit_points`, `k = 0`. This is the "likely downed" test `heal_plan` already uses, and dropping to 0 HP ends concentration (ticket 071).
   - Otherwise every counted opponent's attacks are `d / a` hits of size `a` (expected damage over the average hit, both from `roll_value`). Each hit forces a Constitution save at DC `max(10, floor(a / 2))`, as the engine rolls it. A save effect is one instance at DC `max(10, floor(s / 2))`. `k` is the product of `(1 - p_fail)` over the instances, with a fractional hit count used as an exponent.
   - `p_fail` is the caster's chance to fail the Constitution save. It uses the save-chance helper ticket 073 builds from the engine's sources (stat block save, `save_modifier`, `save_disposition`). It is `p` straight, `p²` with Advantage and `1 - (1 - p)²` with Disadvantage.
3. **Weighted rounds.** The rounds a modifier counts are `Σ_{i=1..ceil(h)} k^(i - f) × clamp(h - (i - 1), 0, 1)`. `f` is 1 when the bearer's next turn comes before any able opponent's next turn in `initiative_order` after the actor, so the first round's benefit lands before anything can break concentration. Otherwise `f` is 0, and always 0 for the actor's own modifiers. With `k = 1` and `h = 2` this is today's 2, so a safe caster in a long fight scores Haste as policy v3 does now. Every part of the bearer value (AC, save Advantage, attack bonus, extra action) uses these weighted rounds in place of `rounds`.
4. **Lethargy.** A `concentration` modifier whose `on_end` effects give the bearer a condition that stops it acting (Incapacitated, Paralyzed, Petrified, Stunned, Unconscious, the `can_act` list) costs `threat(bearer) × (1 - k^h)` as negative damage, while the bearer can act now. That is one lost bearer turn, weighted by the chance concentration breaks inside the horizon, inside the rounds the fight has left. 045 and 055 left it out because the horizon assumed a safe caster. Once `k` is in the formula, leaving it out would count a break's lost benefit but not its lost turn.
5. **Policy version.** `POLICY_VERSION` stays 3. No baseline was ever generated under v3 (ticket 074 is the first), so a v4 would only add a stale-reason step. The build bumps the workspace version one patch. The curve isn't regenerated, and 074 waits on 076.

Measurement, 100 fights per cell, seed 7, release builds with `HOME=$(mktemp -d)`. The three columns are pre-056 (`3f89ef6^1`, no Haste), master (3f89ef6) and the prototype. Each cell gives wins, PC deaths, monster DPR mean and rounds mean:

- adult red dragon L11: 34/66/94.57/5.09, 35/65/101.78/4.95, **44/56/94.90/4.75**. Haste casts 140 on master and 47 on the prototype, with 48 extra attacks.
- adult red dragon L17: 100/0/113.22/2.91, 92/8/108.68/4.34, **99/1/114.28/2.94**. Haste casts 200 on master and 6 on the prototype.
- young red dragon L11: 100/0/68.39/3.02, 98/2/70.75/3.34, 100/0/68.19/3.00.
- ogre and knight at 5, 11 and 17: the prototype is identical to pre-056 in every cell (for example knight L17 rounds 1.07, DPR 3.86) and casts Haste 0 times.
- troll L5, hill giant L5, owlbear L5, stone golem L11, clay golem L11, mammoth L5, fire giant L11, adult red dragon L5: the prototype equals pre-056. Master shortened troll (3.83 to 3.32 rounds), hill giant and stone golem fights but raised monster DPR, and wins and deaths didn't move.

Worked numbers. Troll at level 5: the troll ranks the caster first. Its Multiattack is 27.5 expected damage on the caster, about 2.5 hits of 11, each at DC 10 against Con +2, so `k = 0.34`. `rounds_left = 94 / 52.0 = 1.81` and the striker acts before the troll (`f = 1`), so the weighted rounds are `1 + 0.34 × 0.81 = 1.28` instead of 2. The value is `15.0 × 1.28` for the extra action, plus `20.8 × 0.2 × 1.28` for AC, minus `15.0 × (1 - 0.34^1.81) = 12.9` for lethargy, about 11.6 against master's 38.3. Sleep (31.3) wins the turn. At the adult red dragon's first turn its breath (about 53 on the caster) sets DC 26, so `k = 0` and Haste scores below zero. Once the breath is spent and another PC is the dragon's top target, `k = 1` and Haste comes back. That case is where the 44 wins at level 11 come from.

Ablations on the prototype: rounds left alone fixes the ogre and knight cells, and the keep chance adds the dragon level 11 gain (rounds left alone gives 37 wins). Lethargy changed no measured cell, because the other two already stop those casts, but it is kept so a homebrew `on_end` that stuns or incapacitates isn't free.

Readings chosen:

- Opponents spread their attacks the way the policy itself would pick targets, instead of every opponent attacking the bearer as the AC value assumes. A caster that the monster ranks first is the case where Haste broke on master.
- Hits count as `d / a` instances at the average hit's DC, not per-attack dice. This is close enough to the engine's per-hit save and needs no new walk.
- The rounds-left estimate overstates the party's damage (each PC's best ready action every round), so it errs toward longer fights and more Haste, not less.
- Domain term: the glossary gains **Buff horizon**.

Left out: a shared target model for the AC and save-Advantage values (they still assume every opponent aims at the bearer), and keep chances for other people's concentration (an ally's Haste on the actor). Neither moves a measured cell.
