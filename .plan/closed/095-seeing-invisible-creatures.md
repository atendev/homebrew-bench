---
id: 095
title: Seeing Invisible creatures by spell or trait
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

See Invisibility and homebrew "sees invisible creatures" traits have no effect in the vocabulary, so ADR 0017's sight rule counts only Blindsight and Truesight and the agent overrides the rest. Should seeing Invisible be a trait effect, a timed modifier, or both, and does the standard party ever cast it?

## Notes

- Source: [Align attack-roll condition sources](../closed/067-align-attack-roll-condition-sources-with-2024-rules.md).
- Related: [091](091-darkness-and-darkvision.md) shares the sight rule in ADR 0017. Read both before building either.
- Check `data/party/standard-spells.json` for whether See Invisibility is on the list.
- Skills: `grilling`, `domain-modeling`. If the answer is code, the build is a follow-up `task` ticket.

## Resolution

Resolved in auto mode on 2026-11-07. The main thread asked the questions and a grilling subagent made the decisions. It accepted every recommendation.

Facts, checked against the repo's data:

- No monster in `data/srd/2024/5e-SRD-Monsters.json` mentions seeing Invisible creatures or See Invisibility in any trait, action or spell list.
- The fixtures in `data/fixtures/` that can make a monster Invisible all leave it unmodeled. The imp, quasit, sprite and oni cast Invisibility on themselves. The planetar casts it through Divine Aid, and the adult and ancient blue dragons through the Cloaked Flight legendary action. The will-o'-wisp's Vanish bonus action makes it Invisible, and the invisible stalker has an Invisibility trait.
- `data/party/standard-spells.json` gives the caster Invisibility (tier 2 buff) and Greater Invisibility (tier 4 buff), and not See Invisibility. The policy's `condition_factor` scores Invisible at 0, so it never casts either.
- See Invisibility (PHB 2024, p. 313) has a range of Self and lasts 1 hour without concentration.

So seeing Invisible moves no batch numbers today.

1. **Model it in the engine.** The live-mode advice to remove Invisible by override changes it for every observer, and the Opportunity Attack window and the close-combat check can't be overridden per observer. This fixes an existing rule. It doesn't add a trigger, so the 092 batch-numbers test doesn't apply.
2. **Both carriers, one capability, named `see_invisible`.** A unit trait effect (like `no_provoke`) for "sees invisible creatures" traits, and a `ModifierTarget` `{"see_invisible": null}` with change `none` (like `extra_action`) for the spell. See Invisibility enters `data/spells/effects.json` as a self-bearer modifier lasting 1 hour without concentration. Schema stays 1.
3. **Invisible only.** Blinded and Darkness still block it. It isn't Truesight. The Ethereal Plane clause stays an agent clause.
4. **The standard party doesn't cast it.** The policy can't value sight, nothing is Invisible in batch, and a party that always had it would hide how hard an Invisible homebrew monster is.
5. **Policy reads sight through the shared rule.** Both carriers score 0 as a plan. Policy stays v4. The modifier joins the "Unscored modifiers" fog item.
6. **No new event text.** Notes name the sources that apply. Blindsight and Truesight aren't named when they cancel Invisible either.
7. **The skill lists `see_invisible`** for that trait text and says Truesight already covers it.
8. **Invisible monsters in batch go to fog:** a carrier for a permanent or self-cast Invisible, and whether the party answers it.

ADR 0017 and `CONTEXT.md` (new term "See Invisible") are updated. The build is [105 Build See Invisible](../tickets/105-build-see-invisible.md), blocked by 100 because both edit `fn sees`.
