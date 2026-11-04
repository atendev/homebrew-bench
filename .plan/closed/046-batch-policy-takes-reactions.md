---
id: 046
title: Batch policy takes reactions
type: grilling
mode: HITL
status: closed
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

Decided on 2026-11-04 by a subagent the user delegated the decision to ("use subagents for making decisions"). It grilled itself one sub-question at a time against ADRs 0005, 0012, 0015 and 0016, tickets 044, 045, 047, 053, 054, 056 and 062, `bench-core/src/tactics.rs`, `engine.rs` and `batch.rs`, and the 2024 PHB (Opportunity Attacks p. 370, Reaction p. 371, Shield p. 315). Build ticket: [Build batch reactions policy](../tickets/063-build-batch-reactions-policy.md).

**Facts that shaped it.**
- Only two triggers reach batch: leaving reach (the Opportunity Attack) and being hit by an attack roll (Parry on 7 fixtures, Riposte, and the standard caster's Shield). Every other reaction is skipped or agent judgment (ticket 047).
- 23 of 341 fixtures have a reaction, and none of the 43 dragon fixtures do. For dragons, reactions mean Opportunity Attacks in both directions and the party caster's Shield against their attacks.
- Ticket 062 gates every window on `settings.reaction_windows`, and batch leaves it `false`. In that state the engine never asks the batch decider. Replacing the decider alone changes nothing.
- The v2 step-out penalty in `plan_for` (`tactics.rs` around line 470) uses `best_single_attack`, which counts ranged attacks and ignores Disengage, no-provoke traits and the reactor's sight. The engine never delivers that damage, because batch passes an empty `opportunity` list.
- A targeted move to Engaged breaks no engagement (`move_combatant`), so melee plans and the fallback never provoke. Only the ranged step-out leaves reach.
- Nimble Escape and Cunning Action import as unmodeled and skipped, so no fixture can Disengage as a Bonus Action.
- Shield lasts until the start of the caster's next turn and counts against the triggering attack. The Opportunity Attack is one melee attack, made right before the mover leaves reach.

**Decisions.**

1. **Batch opens windows.** Batch encounters set `reaction_windows` to `true` and install the policy as the decider. *Reason:* with the 062 gate off, the decider is never asked. Batch states are never refolded from a command history, so the replay hazard behind the gate does not apply.
2. **Hit windows: take a reaction only when it turns this hit into a miss.** The policy takes a hit reaction when its AC bonus n gives attack total < AC + n and the d20 is not a natural 20. Otherwise it passes. *Reason:* the window opens after the hit is known, so the reactor sees the total, as the live prompt shows it. Turning a known hit into a miss is how Shield and Parry get used at a table. A Shield taken for a hit it can't stop only pays off on later attacks, and that guess is left out.
3. **Several options in one window.** In a hit window, among the options that turn the hit into a miss, take the cheapest: one with no usage limit or slot first, then the lowest slot, then stat-block order. In an Opportunity Attack window, take the at-will melee attack with the highest expected damage against the mover, with ties in stat-block order. *Reason:* the resource cost weight is 0.0 (ADR 0012), so every option that stops the hit scores the same, and the cost only breaks the tie. Ticket 044 asked for the melee attack picked by expected damage.
4. **Opportunity Attacks: take one when it is worth more than the reaction reserve.** The attack's value is its expected damage against the mover, capped at the mover's hit points, plus the mover's threat when that damage would drop it. This matches how `plan_for` scores a likely kill. The reaction reserve is `0.1 × n × incoming(reactor)` for the reactor's best ready hit-trigger reaction, where "ready" means usage and slot are available, and it is 0 when there is none. The attack is taken when its value is strictly greater than the reserve. *Reason:* a reaction spent on the attack can't stop a hit before the reactor's next turn. The 0.1 per AC point is the value ticket 045 gave an AC buff over one round. A d20 lands in an n-point band 5n% of the time, and that works out near 0.1n of the expected incoming damage. With no hit reaction the reserve is 0, so a monster without Parry always attacks.
5. **Windows are decided one at a time.** Each window is decided on the state when the engine asks, in the engine's order (initiative order for one move, as tickets 044 and 062 set). The policy does not look ahead to later windows. *Reason:* ADR 0012's policy scores the current state. Greedy order already fits the 2024 rule of one reaction until the start of your next turn.
6. **The mover prices the attacks it will really take.** The step-out penalty sums the Opportunity Attack value from decision 4 over reactors that pass the engine's eligibility rule and that the policy would attack with. It no longer uses `best_single_attack`. Ticket 056's teleport score, the Opportunity Attacks a teleport avoids (ticket 045), reads the same sum. *Reason:* ticket 044 asked that the penalty use the engine's melee choice. One rule on both sides keeps ADR 0012's "same policy for both sides" and keeps the penalty equal to the damage batch delivers.
7. **No Disengage plans.** The policy never takes the Disengage action. *Reason:* Disengage spends the action, and a single-monster fight has no goal that retreat serves. Bonus-action Disengage (Nimble Escape, Cunning Action) is unmodeled, so there is nothing cheaper to score.
8. **No new policy constants.** The 0.1 per AC point becomes a named code constant shared with ticket 056's buff scoring. Nothing new goes under `policy.*`. *Reason:* ticket 045 made its values code constants recorded in ADR 0012. A reaction switch would add a standing assumption with no evidence anyone needs it.
9. **One version bump, one regeneration.** The build ticket for this decision moves `POLICY_VERSION` from 2 to 3. Ticket 056 adds its scoring under v3 without bumping again, and it still regenerates the curve once after both land. *Reason:* ADR 0012 says scoring changes bump the version, so a build that changes batch decisions can't land under v2. Neither ticket ships a release in between, so v3 still means one version and one curve.
10. **Its own build ticket, before 056.** The reactions build is ticket 063, blocked by this ticket and 062. Ticket 056 now also waits on 063. *Reason:* 056 already holds three formulas, the Advantage fix, a Haste measurement and the full regeneration. Adding the decider, the batch gate, the penalty fix and their tests would not fit one session.

**Left out, and why.**
- Taking Shield on a hit it can't stop, to protect against later attacks. It needs a forecast of attacks to come. The flip rule already captures most of Shield's value.
- A melee filter on the reserve. Parry only stops melee hits, but the reserve counts all incoming damage. The difference is small next to the d20 band estimate, and a melee-only threat function would be new code for 7 fixtures.
- Limited-use melee attacks as Opportunity Attacks. Ticket 044 allows them only when named, and no fixture has one where it matters.
- Disengage as a plan, and bonus-action Disengage (Nimble Escape on 8 fixtures). See the new fog item in `map.md`.
- Reactions on the other four ADR 0005 triggers, Protective Magic, Whirlwind of Sand and Deflect Missile. They have no window (ticket 047).
- Spreading out against breath weapons. It is an area-count question, not a reaction question. It moves to [Do dragons still read above their CR on the v3 curve](../tickets/064-do-dragons-still-read-above-cr-on-the-v3-curve.md).
