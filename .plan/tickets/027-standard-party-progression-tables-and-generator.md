---
id: 027
title: Standard party progression tables and generator
type: task
mode: AFK
status: open
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
