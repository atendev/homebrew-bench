---
id: 102
title: Hit-triggered Reflexive Antennae and Protective Magic
type: task
mode: AFK
status: open
blocked_by: []
claimed: false
---

## Question

Import the Rust Monster's Reflexive Antennae, and the Shield use of Protective Magic on the Mage, Archmage and Lich, onto the existing `hit_by_attack_roll` trigger.

## Notes

- Source: [Reaction triggers beyond leaving reach and attack rolls](../closed/092-reaction-triggers-beyond-reach-and-attack-rolls.md). No new trigger or vocabulary.
- Reflexive Antennae: `trigger: {hit_by_attack_roll: {attack: any}}` and a `use_action` of `antennae`. All four fixtures today carry their reaction as `kind: unmodeled`, `default_resolution: skipped`.
- Protective Magic stays one entry with its current usage: 3/day on the Mage and Archmage, no limit on the Lich. It gets the hit trigger with `attack: any` and a cast of Shield through the creature's Spellcasting action. The Counterspell use stays an agent clause, so batch spends uses only on Shield. Check that a cast can name a spell missing from the Spellcasting list: Shield is reached only through this reaction.
- Make the change in the SRD import's hand-authored effects layer and regenerate the fixtures, so the import review agrees.
- Batch numbers for the four fixtures change, so the fixture-set fingerprint changes. Regenerate the curve and bump the engine patch version, as ticket 099 did when its fixtures changed.
- [Counterspell and the spell-cast trigger](101-counterspell-and-the-spell-cast-trigger.md) may reshape the Protective Magic entry later. This ticket doesn't wait for it.
- Shield Guardian Protection and Deflect Missile are out of scope.
- Skills: `implement`, `tdd`, `codebase-design`, `domain-modeling`, `code-review`.

## Resolution

<Filled in at close.>
