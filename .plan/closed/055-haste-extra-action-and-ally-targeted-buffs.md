---
id: 055
title: Haste extra action and ally-targeted buffs
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

Should the engine model Haste's extra action and let a timed modifier land on an ally, and if so with what shape? Today modifiers are keyed by the caster only, so Bless-style and Haste-on-ally effects cannot be represented, and Haste's extra action and doubled speed are not modeled.

## Notes

Raised by [Batch policy scores buffs, forced movement and teleports](../closed/045-batch-policy-scores-buffs-forced-movement-and-teleports.md), which scores buffs as self-only. A change would amend ADR 0005 and the effect vocabulary, and would raise Haste's value in the batch policy. Consult `grilling` and `domain-modeling`.

## Resolution

Decided by an agent under the user's delegation, from the code, ADR 0005, ADR 0012, ticket 045 and the 2024 PHB (Haste p. 283, Bless p. 246, Combining Spell Effects p. 236, Speed p. 373).

What the code does today: `resolve_effect` pushes every `TimedModifier` with `source` set to the actor and ignores the targets, and the four readers (`attack_modifier`, `save_modifier`, `ac_modifier`, `apply_modifier_target_disposition`, plus the Reactions check) treat `source` as the creature that has the modifier. So a modifier in a failed save's `on_failure` lands on the caster: Slow gives the caster -2 AC and no reactions, and Hypnotic Pattern gives the caster Speed 0. No reader looks at Speed modifiers at all: the engine, the tactics policy and live mode each compute walk speed from the stat block alone. No SRD fixture carries a timed modifier (0 of 341; monster Spellcasting is unmodeled), so today only the standard caster's Shield, Haste and Hypnotic Pattern reach this code.

1. **A modifier can land on another creature.** Every timed modifier has a bearer (the combatant whose stats or rolls it changes) and a source (the combatant whose ability made it). Concentration still ends what the source holds. Reason: the source-as-bearer reading is a defect for homebrew riders such as "the target's Speed is halved" or "the target has Disadvantage on attack rolls", which is the destination's core case, and the same field gives ally buffs.
2. **Shape: `TimedModifier.bearer`, `target` (default) or `self`.** A `target` modifier lands on each combatant the effect resolves against, so under a save's `on_failure` or an attack's `on_hit` it lands on that target. A top-level `target` modifier with no target named is refused. `ActiveModifier` gains `bearer`, read as `source` in states saved before the change. Reason: conditions and damage already land on targets, and an explicit `self` keeps the engine from guessing from prose (ADR 0005). Shield is `self`; Haste, Fly, Slow and Hypnotic Pattern are `target`.
3. **Same spell, same bearer, no stacking.** `ActiveModifier` keeps `source_ability`. A new modifier with the same `source_ability` and stat on the same bearer replaces the old one. Reason: the PHB's Combining Spell Effects rule (the most recent of equally potent castings applies), and ticket 045's "already active" check needs `source_ability` on the stored modifier.
4. **A modifier can end on damage.** `TimedModifier.ends_on_damage`, as conditions have. Hypnotic Pattern's Speed 0 sets it. Reason: the spell's effect on a creature ends when it takes damage, and once Speed is read, a woken creature would otherwise stay rooted.
5. **Speed modifiers are read.** One walk-speed function serves the engine, the policy and live mode: the lowest `set` wins; otherwise the stat block's walk speed plus every `add`, then doubled once per `double` and halved once per `halve` (rounding down), never below 0. Grappled and Restrained still give 0. Reason: Haste's doubled Speed, Slow's halving and homebrew Speed riders are inert today in all three copies.
6. **The batch policy picks the bearer.** Under v3, a buff plan values ticket 045's formula for the actor and each living ally (incoming damage or threat read on that bearer), skips a bearer that already has the same `source_ability`, and casts on the best one, ties to the actor and then id. Every ally counts as in range, as for heals. A modifier whose bearer is an opponent scores 0. Reason: at a table the caster hastes the most-threatened or hardest-hitting ally, and debuff values were never decided (ticket 045 valued buffs only).
7. **Haste's extra action and its end-of-spell lethargy stay agent clauses.** Reason: they are party-side only (no fixture or plausible monster ability uses Haste), and modeling them needs a new action-economy slot, a CLI flag, a third batch section and an on-end hook. The baseline compares homebrew and fixtures against the same party, so the gap applies to both sides. The two clauses also pull in opposite directions (the extra attack makes Haste stronger, lethargy weaker).

Where the code goes: a new build, [Build modifier bearers and Speed modifiers](../tickets/065-build-modifier-bearers-and-speed-modifiers.md), for decisions 1 to 5. It should not change batch numbers under v2, because v2 never casts a buff and Hypnotic Pattern's targets are Incapacitated and skip their turns, but it must check that. Decision 6 goes into [Build v3 scoring](../tickets/056-build-v3-scoring-for-buffs-forced-movement-and-teleports.md), which already builds the buff formula, and 056 now waits on 065. No new policy bump (063 holds it) and no extra regeneration (056 holds it).

Left out:
- Dice-valued roll bonuses (Bless, Bane, a homebrew "allies add 1d4" aura): `ModifierChange` has no dice form, and nothing in the standard party casts one. Moved to fog in the map.
- Scoring modifiers that land on opponents (a no-save curse, Slow's riders): ticket 045 valued buffs only. Moved to fog in the map.
- Haste's extra action and lethargy: out of scope for phase 1 (decision 7).
