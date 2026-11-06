---
id: 101
title: Counterspell and the spell-cast trigger
type: grilling
mode: HITL
status: open
blocked_by: []
claimed: false
---

## Question

How do the engine and the batch policy model Counterspell through a spell-cast reaction trigger? That covers the window, the counter effect, and when batch spends a counter. A countered spell's slot isn't expended, so does the caster's action still go?

## Notes

- Source: [Reaction triggers beyond leaving reach and attack rolls](../closed/092-reaction-triggers-beyond-reach-and-attack-rolls.md). It passed that ticket's test: Protective Magic on the Mage (CR 6), Archmage (CR 12) and Lich (CR 21) can cancel a party Fireball or heal, so batch numbers move.
- Today the Counterspell effects file is all agent clauses: interrupt a spell cast within 60 feet; the caster makes a Constitution save; on a failure the spell dissipates and the slot isn't expended.
- Read "within 60 feet" as any combatant (ADR 0017, ticket 092), and "a creature you can see" through the sight rule. Sight-gated targeting is still fog, so decide whether this trigger reads it.
- The window would hold the cast command, as ADR 0018 does for hits. A window also opens when a counter is cast, so decide whether a Counterspell can itself be countered.
- Protective Magic is one entry with one usage (3/day on the Mage and Archmage, no limit on the Lich), a hit-triggered Shield cast, and Counterspell as an agent clause ([Hit-triggered Reflexive Antennae and Protective Magic](102-hit-triggered-reflexive-antennae-and-protective-magic.md)). A reaction entry carries one `trigger`, and `per_day` usage has no `group`. Decide whether the entry splits, carries two triggers, or gains a usage group.
- The standard party has no Counterspell. Decide whether the party caster gets it, which changes the baseline curve.
- Skills: `grilling`, `domain-modeling`. The build is a follow-up `task` ticket.

## Resolution

<Filled in at close.>
