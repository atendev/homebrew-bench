---
id: 103
title: Build dice-valued roll bonuses
type: task
mode: AFK
status: open
blocked_by: [093]
claimed: false
---

## Question

Build the decision in [Dice-valued roll bonuses](../closed/093-dice-valued-roll-bonuses.md): the `add_dice` and `subtract_dice` modifier changes, the die on attack rolls and saves, its event note and player-dice prompt, the policy reading of a die and the new save-bonus score, Bless and Bane in the effects file, and the docs.

## Notes

- Skills: `implement`, `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- `ModifierChange` (`bench-core/src/effect.rs` ~364): add `AddDice(String)` and `SubtractDice(String)`, snake_case, additive at schema version 1. Load checks each string with `parse_dice` and refuses a die on any target but `attack_rolls` and `saving_throws`.
- Engine (`engine.rs`): the attack total (~528, `attack_modifier`) and every save total (`save_modifier` ~2634, which concentration and repeat saves reach) add each applying die. Draw it after the d20s, one per modifier in `active_modifiers` order, through `roll_dice` with the bearer as owner so a PC bearer in ask mode gets a prompt and the `(player dice)` mark. Death saves (`roll_death_saves` ~1945) read only die modifiers on `saving_throws` with no ability and draw the die after their d20; flat modifiers stay off death saves so old encounters replay unchanged; a natural 20 or 1 keeps its meaning there and on attacks. No die on an automatic save failure. Legacy modifiers (saved before 0.1.6 with no `bearer`) stay inert as now; they can't hold a die.
- Potency (`encounter.rs` `ActiveModifier::potency` ~86): a die's potency is its mean (1d4 is 2.5). Make the comparison work for fractional values, so a flat +3 beats a 1d4 and a 1d4 beats a +2. Magnitude decides across signs, as for `add` today.
- Event note: ` (+1d4 Bless: 3)` and ` (-1d4 Bane: 2)`, named by `source_ability`. The public view hides a monster's die result wherever it hides that monster's roll total.
- Policy (`tactics.rs`): read a die as its mean, a fractional flat bonus, wherever the policy reads flat attack and save bonuses (hit chance, `save_success` ~1275). In `bearer_value` (~915), score an attack die like `+n attack` with n its mean, and add a save-bonus score for flat or dice: incoming save damage × `SAVE_POINT_VALUE` (new, 0.05) × n × rounds, for the abilities the modifier reaches, positive bonuses only. Keep the existing 0-to-1 clamps on hit and save chances; the mean is accepted as slightly off where they bind. Bane, Slow's -2 and other negative bonuses score 0 until ticket 094.
- Data (`data/spells/effects.json`): Bless gets concentration, up to three targets, `add_dice 1d4` on `attack_rolls` and on `saving_throws` with no ability, and `extra_targets` 1 per slot level above 1; drop its agent clause. Add Bane: a Charisma save whose `on_failure` holds `subtract_dice 1d4` on both, concentration, the same upcast. Divine Aid on the priest, priest-acolyte and couatl stays unmodeled, so no fixture changes.
- Tooling: `scripts/check-spell-effects.py` and `scripts/scaffold-spells.py` accept the new variants, and `bench schema` shows them. The import review is unchanged.
- Tests: a die on attacks and on saves (including a concentration save and a death save, and a flat no-ability save modifier leaving a death save unchanged), Bane landing only on targets that fail, an upcast Bless or Bane taking a fourth target, a mixed-sign potency pair from one source, Bless and Bane on one creature both rolling, two Blesses giving one die, a die refused at load on `armor_class`, an ask-mode PC prompt for the die, the stream unchanged for an encounter with no die, and an old state with flat modifiers refolding to the same hashes.
- Docs: `CONTEXT.md` and ADR 0005 were updated in ticket 093. The live-mode reference in the skill names the die note, the player-dice prompt, and the override that adds an `add_dice` active modifier for an aura the agent applies, cleared by override when the ally leaves the aura or its source stops counting. Add a CHANGELOG line.
- Versions: bump the workspace one patch. `POLICY_VERSION` stays 4. Regenerate the curve; it must differ only in `engine_version`. If any cell moves, stop and report, since the decision then calls for v5.

## Resolution

<Filled in at close.>
