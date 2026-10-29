---
id: 027
title: Standard party progression tables and generator
type: task
mode: AFK
status: closed
blocked_by:
  - 025
  - 026
claimed: false
---

## Question

Author the four designer-set archetype progression tables (`defender`, `striker`, `healer`, `caster`) under `data/party/`, build the generator script in `scripts/`, and commit all 80 standard party stat blocks (levels 1 to 20).

## Notes

Consult `implement`, `tdd`, and `domain-modeling`.

Key decisions to implement:
- **Standard party design (`docs/adr/0010-standard-party-of-four-generated-archetypes.md`, closed tickets 014, 024):**
  - Four roles matching the SRD four-character calibration: `defender` (front line), `striker` (ranged-capable), `healer`, and `caster` (arcane caster). Each stat block notes the classic class it approximates in a comment/note field without transcribing non-SRD rulebook tables.
  - Four committed progression tables under `data/party/` covering levels 1 through 20: HP, AC, ability scores, saving throw proficiencies, attack bonuses, damage dice, multiattack/extra attack progression, class-approximating combat traits/actions, and spell slots.
  - No magic items at any level; mundane gear is baked into stat block numbers and noted in `gear`.
  - PCs carry the PC-only `dying` / death-save behavioral field (unconscious at 0 HP, rolling death saves, healable, killable) and level/class labels.
  - Caster and healer spellcasting blocks reference the spell ids committed in `data/party/standard-spells.json` and modeled in `data/spells/effects.json`.
- **Generator and verification:**
  - A deterministic script in `scripts/` reads the progression tables and `data/party/standard-spells.json` and writes the 80 generated stat blocks under `data/party/`.
  - Automated tests in `bench-core` verify that all 80 generated stat blocks parse cleanly, pass stat block schema validation with zero unmodeled abilities or unresolved spell ids, and match the generator output cleanly.

## Resolution

Delivered the four designer-set tables, the generator, and all 80 standard party stat blocks.

- `data/party/progression/{defender,striker,healer,caster}.json`: step-based tables (`from` level) for ability scores, armor class and source, gear, weapon attacks, precision and divine-strike bonus dice, multiattack counts, features, and spell slots. Hit die, saves and skills are set once per archetype. Each table carries a `class_note` naming the class it approximates.
- `scripts/generate-party.py`: deterministic. Reads the tables, `data/party/standard-spells.json`, `data/spells/effects.json` and SRD casting times, and writes `data/party/standard/standard-<archetype>-<NN>.json`. `--check` fails on missing, differing or unexpected files.
- Spells become `cast` abilities that point at effects-file ids. A spell with agent clauses is `partly_modeled`. Casting time picks the list: action, bonus action or reaction. Spells that take a minute or longer (Prayer of Healing, Raise Dead, Heroes' Feast, Regenerate, Resurrection, True Resurrection, Foresight) are left off the blocks; a test pins that set.
- Slots use the full-caster shape, with spell tiers gated by the highest slot tier. `spell_slots` index 0 is 1st level.
- Schema additions on `StatBlock`: `spellcasting` (ability, save DC, attack bonus) and `notes` (designer notes, no combat effect).
- `bench-core/src/party.rs` holds naming helpers and seven tests: 80 blocks validate, no unmodeled abilities, every cast id resolves, party invariants (death saves, no magic gear, HP growth, slots), hand-computed oracles, tier gating and at-will options, and generator parity.

Design calls: the striker has one attack per action with precision dice on every hit, standing in for a once-per-turn bonus, so it has no extra-attack progression (stated in its `class_note`). Defender, striker, healer and caster use d10, d10, d8 and d6 hit dice with fixed average HP per level.

An audit subagent found no blocking issues. Its low findings are fixed: oracle and tier tests, a pinned skip set, unused parameters, UTF-8 output, striker note.

**Engine gaps the party exposes (not fixed here, belong to later tickets):**
- Bonus actions and reactions have no execution path, so Second Wind, Healing Word, Mass Healing Word, Lesser Restoration and Shield cannot run yet.
- `resolve_cast` ignores `slot_level` and `spell_slots`; slots are never spent and upcasting does nothing.
- `Spellcasting.save_dc` and `attack_bonus` are not read. Effects-file spells carry `dc: 0` and `modifier: 0`, and heals have modifier 0.
- Cantrip dice do not scale at levels 5, 11 and 17.
- A hit on a PC at 0 HP resets death saves instead of adding a failure, and the engine treats `class.is_some()` as "is a PC" rather than reading `dying`.
