---
id: 093
title: Dice-valued roll bonuses
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

Bless, Bane and a homebrew "allies add 1d4 to attack rolls" aura need a modifier that adds a die, and `ModifierChange` has only flat values. Should the change take a dice string, how does the policy value it, and does a die count toward Advantage-style stacking?

## Notes

- Source: [Haste extra action and ally-targeted buffs](../closed/055-haste-extra-action-and-ally-targeted-buffs.md).
- The standard party casts neither Bless nor Bane, so this matters only for homebrew monsters and agent-run live fights.
- Overlaps with [094](094-batch-value-of-debuffs.md): Bane is a debuff die. Settle the dice representation here first, or resolve both in one session.
- Check the replay impact: a new `ModifierChange` shape touches saved state and the log.
- Skills: `grilling`, `domain-modeling`. If the answer is code, the build is a follow-up `task` ticket.

## Resolution

Grilled on 2026-11-06 in the `wayfinder-runner` skill's auto mode (`~/.claude/skills/wayfinder-runner/SKILL.md`, "Auto mode"), which the user turned on for this run: the main thread put the questions and a grilling subagent answered in place of the user. The user reviews the result at the commit gate. It took every recommendation except Q1, where it picked two variants over one signed string.

1. **Representation.** `ModifierChange` gains two variants, `add_dice` and `subtract_dice`. Each holds an `NdM` string that `parse_dice` checks at load. `add(i32)` is untouched, so old saves read as before.
2. **Targets.** A die is accepted only on `attack_rolls` and `saving_throws`, the d20 tests the engine rolls. A die on any other target fails at load. Bless and Bane each carry two timed modifiers, one on attacks and one on all saves.
3. **Stacking.** A die is a bonus, not a disposition, and never enters the Advantage two-flag rule. Bless and Bane on one creature both roll and don't cancel. The one-spell-once-per-bearer rule still applies, with a die's potency read as its mean (1d4 is 2.5), so two Blesses give one d4. Different sources, such as Bless and a homebrew aura, both roll. Potency compares magnitude as it already does for `add` (-3 beats -1), so within one spell a +3 beats a `subtract_dice 1d4`; no SRD or effects-file spell mixes signs on one stat.
4. **Rolls reached.** The die applies to every attack roll and save the engine rolls: Opportunity Attacks, concentration saves and repeat saves included. Death saving throws are saving throws, so a die modifier on `saving_throws` with no ability reaches them too. Flat and dice differ on death saves on purpose: flat modifiers keep not reaching death saves, as today. By the rules they should, but every saved state since 0.1.6 can hold a flat no-ability save modifier with a bearer, so reading them would change death-save outcomes when old encounters refold, while no old state can hold a die. Closing that gap needs a recorded-version gate like ticket 062's `settings.reaction_windows`, and it goes to fog. An automatic save failure rolls no die. A natural 20 or 1 keeps its meaning, on attacks and on death saves.
5. **Who rolls.** The bearer rolls the die, because both spells say the target adds or subtracts it. A PC bearer in ask mode gets a dice prompt and the `(player dice)` mark. The die draws from the same stream right after the d20s, one per applying modifier in `active_modifiers` order, so encounters without one draw exactly as before.
6. **Log and public view.** The event note names each die and its result, as ` (+1d4 Bless: 3)` and ` (-1d4 Bane: 2)`. The public view hides a monster's die result wherever it hides that monster's roll total.
7. **Policy value.** The policy reads a die as its mean, a fractional flat bonus, wherever it reads flat bonuses (hit chance, `save_success`). The chances are linear in the bonus until the existing 0-to-1 clamps bind, so the mean is exact in the middle and slightly off at the edges, which the policy accepts. A new save-bonus buff score covers flat and dice bonuses: incoming save damage × `SAVE_POINT_VALUE` (0.05, one point per 5%) × n × rounds. This closes the old gap where a flat save bonus scored 0. The weight is half of `AC_POINT_VALUE` because a save usually still deals half damage on a success, while a missed attack deals none; for a save that only imposes a condition the score runs low, which the policy accepts. Only positive bonuses score, as for AC and attack bonuses. Bane and other subtracted dice score 0 as debuffs until ticket 094.
8. **Replay.** The variants are additive. Every saved state and replay from earlier engines loads and refolds unchanged. The workspace takes a patch bump, as every engine change in 0.1.x has. The state schema stays at version 1 by ADR 0005's `no_provoke` exception, which already covers new enum variants: the `no_provoke` and `disengage` effects and the `dim_light` lighting value (ticket 060) each arrived in a patch bump at version 1, and older binaries can't read a state holding the new variants.
9. **Data.** The build authors Bless (concentration, three targets, `add_dice 1d4` on attacks and all saves, one more target per slot level above 1) and adds Bane (a Charisma save with `subtract_dice 1d4` on both under `on_failure`, concentration, the same upcast). Divine Aid on the priest, priest-acolyte and couatl stays unmodeled, so no fixture changes.
10. **Policy version.** No curve input casts Bless or Bane or carries a positive save bonus, so `POLICY_VERSION` stays 4. The new save-bonus term fires only on a positive save bonus, and the only flat save bonus in the embedded data is Slow's -2 on Dexterity saves, a debuff, so no embedded input's plan changes. ADR 0012 puts the policy version in baseline fingerprints so a policy change marks baseline cells stale. A homebrew run is compared with the baseline, never with an earlier homebrew run, so a homebrew stat block with a positive save bonus scoring differently doesn't need a new version while no baseline cell moves. Ticket 078 added a scoring rule under the same version on the same reasoning. The curve is regenerated and must differ only in `engine_version`. If any cell moves, the build stops and the policy goes to v5.
11. **Auras.** A homebrew aura that gives other creatures a die is zone-like and stays agent judgment. In live mode the agent adds a permanent `add_dice` modifier on each ally by override and clears it by override when the ally leaves the aura or its source stops counting (dead, Incapacitated, or the aura ends). Batch refuses the ability unless an `abilities.<id>` assumption covers it.
12. **Tooling.** `scripts/check-spell-effects.py` and `scripts/scaffold-spells.py` learn the new variants, and `bench schema` shows them. The import review is unchanged.
13. **Docs.** `CONTEXT.md`'s Modifier entry and ADR 0005 were updated in this ticket. The build updates the live-mode reference. No new ADR, because the change is additive and easy to reverse; ADR 0005's bullet records the rejected single signed string.
14. **Warnings.** The load-time refusal in item 2 covers a die on an unread target, so batch gets no new warning.

This settles the dice representation that ticket 094 needs for Bane. The build is ticket 103.
