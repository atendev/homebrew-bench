---
id: 046
title: Batch policy takes reactions
type: grilling
mode: HITL
status: open
blocked_by: [053, 054, 047]
claimed: false
---

## Question

When should the batch tactics policy spend a reaction, for both sides? Cover opportunity attacks under the rule ticket 044 sets, stat-block reactions on the six reaction triggers (such as Parry), and reaction spells in the standard party list (such as Shield), and decide how one reaction is chosen when several are open at once.

## Notes

v1 takes no reactions (ADR 0012, `bench-core/src/tactics.rs`). The six reaction triggers come from ADR 0005. A change here bumps the policy version and the baseline curve. Consult `grilling` and `domain-modeling`.

Scope added by [Engine-rolled opportunity attacks](../closed/044-engine-rolled-opportunity-attacks.md): batch takes opportunity attacks in initiative order, picks the melee leaf by expected damage, fixes the v2 penalty in `tactics.rs` (it counts ranged attacks and is never paid), and bumps the policy to v3 once with one baseline regeneration. Batch adoption waits on the two build tickets for opportunity attacks and for Disengage and no-provoke traits.

Scope added by [Live reaction windows](../closed/047-live-reaction-windows.md): the engine asks a reaction decider at every window, for both the Opportunity Attack (leaving reach) and being hit by an attack roll. Batch passes every window until this ticket picks a policy, so this ticket only decides what the batch decider returns. Parry and Riposte import as modeled, with the trigger `hit_by_attack_roll {attack: melee}`, and the standard caster's Shield gets `hit_by_attack_roll {attack: any}`. The batch build should wait on [Build live reaction windows](062-build-live-reaction-windows.md).

## Resolution
