---
id: 092
title: Reaction triggers beyond leaving reach and attack rolls
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

ADR 0005 lists four reaction triggers the engine does not model: an attack roll made against the creature, taking damage or becoming Bloodied, a creature moving within N feet, and a creature making a check or save. There is also the dormant spell-cast trigger. Which homebrew reactions need them, and how does "within N feet" read without positions?

## Notes

- Source: [Live reaction windows](047-live-reaction-windows.md), [ADR 0005](../../docs/adr/).
- Nothing modeled uses these triggers yet, so the agent handles them as judgment. The question is whether the cost of modeling any of them is worth it.
- Start from the SRD reactions that would use each trigger (Shield, Parry, Uncanny Dodge, Counterspell) and the homebrew patterns the user pastes most.
- Skills: `grilling`, `domain-modeling`. If the answer is code, the build is a follow-up `task` ticket.

## Resolution

Resolved on 2026-11-06 in an auto-mode grilling run by `wayfinder-runner`: a subagent answered for the user, and the user reviews the decisions at the commit gate.

**The test.** A reaction trigger is worth modeling only when a modeled or importable reaction needs it and it moves batch numbers. Live mode already copes without one: the agent runs the reaction off-turn with `bench action` or an override, so it loses only a reminder. In batch, an unmodeled reaction never fires.

**Verdicts, from the 24 SRD monster reactions.** The reactions the notes name first are already settled. Shield and Parry use the hit trigger (ADR 0018). Uncanny Dodge is a hit trigger too, but its halved damage has no effect in the vocabulary, and no standard party member has it. That leaves Counterspell, the first verdict below.

- **A spell being cast passes.** The Mage, Archmage and Lich can Counterspell a party Fireball or heal through Protective Magic. It is not ready to build: when the batch policy spends a counter, and whether a countered spell still spends the caster's action, are open. Next is the grilling ticket [Counterspell and the spell-cast trigger](../tickets/101-counterspell-and-the-spell-cast-trigger.md).
- **An attack roll made against it fails.** Only the Goblin Boss's Redirect Attack uses it, and that needs a swap-places and retarget effect the vocabulary lacks. It stays agent judgment.
- **Taking damage or becoming Bloodied fails.** Split spawns creatures and is skipped by default, and Ink Cloud is an underwater zone. A homebrew retaliation reaction stays agent judgment through the resolution batch already requires.
- **A creature moving or ending its move within N feet fails.** The Shrieker has no combat effect, Ink Cloud is underwater, and the Nalfeshnee's Pursuit teleport changes little when range bands stay unchanged.
- **A check or save fails.** The Sphinx of Wonder's +2, twice a day, barely moves the numbers, and the engine rolls no checks.
- **A creature starting its turn nearby** is not on ADR 0005's list. The Chain Devil's Unnerving Gaze uses it. It fails for now: one fixture, a small effect, and it needs an immunity on a successful save, which is its own question.

**"Within N feet" without positions.** For any future trigger: N of 5 or less reads as engaged (ADR 0017), and N over 5 reads as any combatant in the encounter, as every sense reaches every band. Range bands have no width, so a band reading was rejected again. Nothing is built on it now. ADR 0017 records the reading.

**Hit-triggered reactions the existing trigger can carry.** The ticket [Hit-triggered Reflexive Antennae and Protective Magic](../tickets/102-hit-triggered-reflexive-antennae-and-protective-magic.md) imports them, with no new trigger. The party caster's Shield already has this trigger; the ticket extends it to two monster reactions:

- Reflexive Antennae gets `hit_by_attack_roll` and uses Antennae.
- Protective Magic stays one entry, so the Mage's and Archmage's 3/day stays shared (the Lich's has no limit). It gets the hit trigger and a cast of Shield, and the Counterspell use stays an agent clause, so batch spends uses only on Shield. `per_day` usage has no `group` key, so a split would need new vocabulary. The Counterspell ticket decides whether the entry splits, carries two triggers, or gains a usage group.

Shield Guardian Protection (a hit on another creature) and Deflect Missile (damage reduction) don't fit the existing trigger and stay as they are.

No engine code, no new ADR (only notes in ADR 0005 and ADR 0017), and no `CONTEXT.md` change: the Reaction trigger entry still says exactly which two triggers open windows.
