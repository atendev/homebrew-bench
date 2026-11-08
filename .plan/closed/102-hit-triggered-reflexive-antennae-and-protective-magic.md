---
id: 102
title: Hit-triggered Reflexive Antennae and Protective Magic
type: task
mode: AFK
status: closed
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
- [Counterspell and the spell-cast trigger](../closed/101-counterspell-and-the-spell-cast-trigger.md) decided to split the Protective Magic entry later, in [Build batch Counterspell and the Protective Magic split](111-build-batch-counterspell-and-the-protective-magic-split.md). This ticket keeps the entry whole and doesn't wait for it.
- Shield Guardian Protection and Deflect Missile are out of scope.
- Skills: `implement`, `tdd`, `codebase-design`, `domain-modeling`, `code-review`.

## Resolution

Built. The importer has two new prose patterns beside Parry, and the fixtures, import review and curve are regenerated. Engine 0.1.29, policy stays v4. No engine code changed, because `cast_spell` reads the spell effects file and never a Spellcasting list. Two engine tests and two fixture tests cover it.

**Protective Magic.** The Mage, Archmage and Lich carry the hit trigger with `attack: any` and a `cast` of Shield with no slot level. Usage is unchanged: 3/day, none on the Lich. The Counterspell use is an agent clause with no `default_resolution`, as Riposte's is, so `bench batch` on those three now needs `--assume abilities.protective-magic=ignored` until ticket 111 splits the entry.

**Reflexive Antennae.** It carries the hit trigger and a `use_action` of `antennae`, as the ticket says. It imports as `partly_modeled` with an agent clause and keeps `default_resolution: skipped`, because the Antennae action has no effects. The auditor raised this. It opens a window in live mode and moves no batch number, against the ticket's expectation that all four fixtures change.

**Curve.** The Mage moves in 15 cells, the Archmage in 19 and the Lich in 19. The Mage is harder: the party wins 66 of 100 at level 3, down from 85.

The decision report is [102-implementation](../reports/102-implementation.md).
