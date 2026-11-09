---
id: 101
title: Counterspell and the spell-cast trigger
type: grilling
mode: HITL
status: closed
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
- Protective Magic is one entry with one usage (3/day on the Mage and Archmage, no limit on the Lich), a hit-triggered Shield cast, and Counterspell as an agent clause ([Hit-triggered Reflexive Antennae and Protective Magic](../tickets/102-hit-triggered-reflexive-antennae-and-protective-magic.md)). A reaction entry carries one `trigger`, and `per_day` usage has no `group`. Decide whether the entry splits, carries two triggers, or gains a usage group.
- The standard party has no Counterspell. Decide whether the party caster gets it, which changes the baseline curve.
- Skills: `grilling`, `domain-modeling`. The build is a follow-up `task` ticket.

## Resolution

Resolved on 2026-11-08 in an auto-mode grilling run by `wayfinder-runner`: a subagent answered for the user, and the user reviews the decisions at the commit gate. The design is recorded in [ADR 0019](../../docs/adr/0019-counterspell-through-a-spell-cast-window.md).

- **Window.** A third trigger, `spell_cast`, holds the cast of an enemy's leveled spell, one window per reactor in initiative order. The first taken counter that succeeds ends the cast. Cantrips and allies' casts open no window.
- **Effect.** A `counter_spell` effect: the caster makes a Constitution save against the reactor's spell save DC, and a failure cancels the cast. Counterspell is cast at level 3.
- **Cost of a countered cast.** The action stays spent, the slot is not expended, and a per-day or innate usage stays spent.
- **Limits.** Sight is not read, and a reaction cast opens no window, so counters are not countered by the engine. Live mode can handle that with `bench action`.
- **Batch.** A reactor counters when the cast's expected value beats the reaction reserve plus the slot or usage cost, with a logged reason, and never for a zero-value spell.
- **Protective Magic.** It splits into a Shield entry and a Counterspell entry sharing a `group` key added to `per_day` usage.
- **Party and gate.** The party caster gets no Counterspell. A new `settings.spell_cast_windows` gate keeps old saves unchanged, with engine and policy version bumps.

Build tickets: [Build the spell-cast window and counter_spell effect](../closed/110-build-the-spell-cast-window-and-counter-spell-effect.md) and [Build batch Counterspell and the Protective Magic split](111-build-batch-counterspell-and-the-protective-magic-split.md).
