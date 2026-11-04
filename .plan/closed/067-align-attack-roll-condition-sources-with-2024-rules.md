---
id: 067
title: Align attack-roll condition sources with the 2024 conditions
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

Which of the gaps between the engine's attack-roll Advantage and Disadvantage sources and the 2024 conditions should the engine close, how should the position-dependent ones read without a grid, and should the fix land before ticket 056 regenerates the baseline?

## Notes

Found while building [Fix Advantage and Disadvantage cancellation](../closed/061-fix-advantage-disadvantage-cancellation.md). `disposition` in `bench-core/src/engine.rs` feeds `combined_disposition`, so a new source is one more entry in its list.

Clear-cut gaps (PHB Appendix C):

- Blinded target: attack rolls against it have Advantage (p. 360).
- Restrained attacker: its attack rolls have Disadvantage (p. 372).
- Petrified target: attack rolls against it have Advantage (p. 371).

Gaps that need a reading without positions:

- Grappled attacker: Disadvantage against any target other than the grappler (p. 366). `ConditionInstance.source` could name the grappler.
- Prone target: Advantage only when the attacker is within 5 feet, else Disadvantage (p. 371). The engine always gives Advantage. `engaged` could stand for within 5 feet.
- Frightened attacker: Disadvantage only while the source is in line of sight. The engine always applies it.
- Invisible: the engine ignores Blindsight, Truesight and See Invisibility on the other side.

Any fix changes batch numbers and moves the engine one patch. If it should share ticket 056's regeneration, 056 needs this ticket in `blocked_by`. The tactics policy's own `disposition` in `tactics.rs` has the same list, and 056 rewrites it under v3.

## Resolution

Decided on 2026-11-04 by a delegated agent, under the user's instruction to make the decision without coming back. Evidence: ADR 0003, 0005, 0015 and 0012, closed tickets 049 and 061, `disposition`, `opportunity_eligibility`, `move_combatant`, `apply_damage` and `retarget_if_down` in `bench-core/src/engine.rs`, `ConditionInstance` and `Senses`, the tactics policy's `band` and `disposition` in `tactics.rs`, and PHB Appendix C: Blinded (p. 360), Blindsight (p. 360), Grappled (p. 366), Frightened (p. 366), Invisible (p. 369), Paralyzed (p. 370), Petrified (p. 371), Prone (p. 371), Restrained (p. 372), Truesight (p. 376) and Unconscious (p. 376), with "Dropping to 0 Hit Points" (ch. 1, p. 27) and See Invisibility (ch. 7, p. 313).

What the code does today: the attack disposition reads the actor's Blinded, Frightened, Poisoned and Prone as Disadvantage, the target's Paralyzed, Prone, Restrained, Stunned and Unconscious as Advantage, and Invisible on either side, with no reference to who applied a condition, where anyone stands, or what anyone can see. A condition pushed by an effect records the applying combatant in `source`. Engagement is a symmetric list each combatant holds, set when one moves to the engaged band and cleared when it leaves. A PC at 0 HP has `end_state` Unconscious or Stable but no Unconscious condition, so attacks against it roll straight and never crit automatically. No hit is ever a critical except on a natural 20. The Opportunity Attack check (ADR 0015) assumes Invisible hides the mover from every reactor and a Blinded reactor sees nothing.

- **Close all seven gaps in one build.** Blinded target and Petrified target give Advantage; Restrained attacker gives Disadvantage. These three need no reading. Reason: they are plain rules text, and one build with one engine bump costs less than several.
- **"Within 5 feet" reads as engaged.** The attacker is within 5 feet of the target when either holds the other in its engagement list. Reason: engagement already stands for melee reach for Opportunity Attacks (ADR 0015), and the tactics policy's `band` reads it the same way. The range band alone is not used, because a missing band defaults to engaged in the engine. A reach weapon used from 10 feet reads as within 5 feet; that error is accepted.
- **Prone target: Advantage when engaged, otherwise Disadvantage.** Ranged attacks from the near or far band now roll with Disadvantage against a Prone target, and a ranged attack from an engaged attacker gets Advantage, as the rule says.
- **Grappled attacker: Disadvantage unless the target is the condition's `source`.** A Grappled condition with no source (an override that didn't name one) gives Disadvantage against every target, since no target can be shown to be the grappler. Reason: the engine already records the applier in `source`, and ADR 0005 already tells the agent to name the grappler there.
- **One sight rule.** An observer sees another combatant unless the other is Invisible and the observer has neither Blindsight nor Truesight, or the observer is Blinded and has no Blindsight. Sense ranges don't count: every sense reaches every band. Line of sight is otherwise assumed, because nothing blocks it without positions. Tremorsense doesn't see. Reason: comparing sense ranges with range bands is out of scope on the map (ticket 049), so the faithful reading is "has the sense". The rule then serves all three places sight matters:
  - Invisible attacker: Advantage only against a target that doesn't see it. Invisible target: Disadvantage only for an attacker that doesn't see it.
  - Frightened attacker: Disadvantage while it sees the condition's `source`, whatever the source's end state, because a corpse is still in line of sight. A source that has left the encounter (`bench remove`) no longer counts. A Frightened condition with no source always gives Disadvantage, as today.
  - Opportunity Attack eligibility uses the same rule, so a reactor with Blindsight or Truesight can strike an Invisible mover and a Blinded reactor with Blindsight can react. ADR 0015's sentence on sight is amended.
- **Blinded keeps its own attack effects.** A Blinded attacker still has Disadvantage and a Blinded target still grants Advantage even when it has Blindsight. Reason: the condition's "Attacks Affected" text doesn't depend on sight, and reading Blindsight into it is a table ruling the agent can make with an override.
- **Damage to a Stable PC.** The audit found that `apply_damage` treats only a dying PC as down, so a Stable PC takes damage to its HP and stays Stable. By the PHB (ch. 1, Stabilizing a Character, pp. 27-28) it takes a death save failure, two on a critical, and stops being Stable. Ticket 068 fixes it with the automatic critical hits.
- **A downed PC counts as Unconscious, and Unconscious counts as Prone, for attacks.** A combatant whose `end_state` is Unconscious or Stable is treated as having the Unconscious condition, and the Unconscious condition (or downed state) is treated as Prone. Nothing is pushed into `conditions`; the attack sources read it. So a downed or Unconscious target gives Advantage when engaged and a straight roll from range (Advantage from Unconscious, Disadvantage from Prone). Reason: the PHB gives the condition at 0 HP and the Prone condition inside Unconscious; storing them would change state files, overrides and the digest for no gain.
- **Automatic critical hits.** A hit against a Paralyzed or Unconscious (or downed) target is a Critical Hit when the attacker is engaged with it. A downed PC hit this way takes two death save failures, which `apply_damage` already does for a critical. Reason: it is the same "within 5 feet" reading and the same two conditions, and ghoul paralysis and Hold Person turn on it. It shifts the dice stream (extra damage dice), which is fine because batch numbers move anyway.
- **Land before ticket 056's regeneration.** Batch numbers change wherever a fight has a Restrained or Grappled attacker, a Blinded, Petrified, Prone or Paralyzed target, or an Invisible combatant facing Blindsight or Truesight. The build bumps the engine one patch and leaves the curve stale. Ticket 056 waits on the build ticket instead of on this one.
- **The tactics policy needs the same sources.** Its `disposition` in `tactics.rs` carries its own copy of the list, so v3 would score Prone, Grappled and Invisible attacks differently from how the engine rolls them. Ticket 056 already rewrites it under v3; it should call the engine's attack disposition instead of keeping a second list, and read engagement as the plan leaves it. That is noted in 056. The build ticket doesn't touch `tactics.rs`, so `POLICY_VERSION` doesn't move here.

Left out:

- See Invisibility and homebrew "sees invisible creatures" traits. Nothing in the effect vocabulary says a combatant sees Invisible creatures, so the agent removes the Invisible condition by override when such a creature attacks. Recorded in Not yet specified.
- Condition effects on saving throws: Paralyzed, Stunned, Petrified and Unconscious auto-fail Strength and Dexterity saves, and Restrained has Disadvantage on Dexterity saves. The engine applies none of them. They move batch numbers too, so after the audit they became build ticket [069](../tickets/069-build-saving-throw-condition-sources.md), which waits on ticket 060's save disposition and lands before 056's regeneration.
- Grapples ending when the grappler is Incapacitated, and Frightened's "can't approach" movement rule. These change when conditions end or where creatures move, not attack rolls.
- Darkness and Heavily Obscured areas. The Darkness fog item stays, and now has its sense-range question answered by this decision.

Build tickets: [Build attack-roll condition sources](../tickets/068-build-attack-roll-condition-sources.md) and [Build saving-throw condition sources](../tickets/069-build-saving-throw-condition-sources.md). ADR: [0017](../../docs/adr/0017-position-worded-rules-read-through-engagement-and-senses.md).
