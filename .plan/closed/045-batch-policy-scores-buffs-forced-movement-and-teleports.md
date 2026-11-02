---
id: 045
title: Batch policy scores buffs, forced movement and teleports
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

How should the batch tactics policy score plans that use buffs (timed modifiers on allies), forced movement, and teleports, which v1 scores at zero? Decide a value for each effect kind in the same units the policy already uses for damage and control, and whether a v3 policy changes any existing SRD fixture's results enough to need a regenerated curve.

## Notes

The policy lives in `bench-core/src/tactics.rs` and ADR 0012. A monster built around these effects reads weaker in batch reports than it plays. Any scoring change bumps the policy version and makes the committed baseline stale until `bench baseline generate` reruns. Consult `grilling` and `domain-modeling`.

## Resolution

Decided with a drafted proposal from a subagent, approved by the user. All values are in the policy's existing unit, expected HP, at weight 1.0.

1. **Buffs (`TimedModifier`)** score as self-only incoming-damage reduction in `survival`: `incoming(actor) x reduction x min(rounds, buff_horizon)`, with `buff_horizon = 2` a new named constant. Reduction: AC +n is 0.1 n, advantage on saves or disadvantage on foes' attacks is 0.3, an attack-roll bonus +n scores the actor's own threat x 0.1 n, and speed, reactions and damage rolls score 0. A concentration duration counts as the horizon. A buff scores 0 when the same `source_ability` modifier is already active, and `breaks_own_concentration` still applies. Self-only because the engine keys modifiers by the caster (`engine.rs:382-390`); ally-targeted buffs are not representable today.
2. **Forced movement** scores as `control`: `threat(target) x f`. f is 0.25 for a push of 5 feet or more when every attack of the target needs engagement, 1.0 for a push of 35 feet or more to Far when the target's walk speed is under 35, and 0 for a pull or a ranged-only target.
3. **Teleport** scores in `survival` only when the actor is engaged: the sum of `best_single_attack` over engaged foes that still have their reaction, the opportunity attacks avoided. It scores 0 when not engaged and is never treated as a free move that unlocks a melee plan.
4. **Policy v3 and baseline.** No SRD fixture uses these effect kinds on an action (0 of 341), so monsters do not change. The standard party casts Haste at caster levels 5 to 20 and would start scoring it, so party cells may shift and the effect must be measured, not assumed. The scoring change ships in policy v3 together with the reactions change, and the baseline is regenerated once, in the reactions build.
5. The values are code constants, not `policy.*` overrides. They are recorded in ADR 0012 (amended by the build ticket).

Left out on purpose: Haste's extra action and ally-targeted buffs, which are a separate engine question (see [Haste extra action and ally-targeted buffs](../tickets/055-haste-extra-action-and-ally-targeted-buffs.md)).
