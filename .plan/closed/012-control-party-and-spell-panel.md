---
id: 012
title: Control party and spell panel
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Build the control run and the spell panel from [Spell value metric](../closed/001-spell-value-metric.md) ([ADR 0020](../../docs/adr/0020-spell-lift-as-the-spell-value-metric.md)): the standard party with the caster role's spells of one level removed, and per party level a fingerprinted panel of fixtures it wins about 40 to 80% of the time.

## Notes

- Skills: `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- Control transform: take the standard party at a party level and remove every spell of level L from the caster role's stat block. The slot table stays. The rest of its list stays.
- Panel selection: from the baseline-curve fixtures (`baseline::embedded_fixtures`), run the control party and keep fixtures in the 40 to 80% win band. Give each panel a single-monster shape and a group shape (several copies of a lower-CR fixture). The single-monster shape includes a spellcasting fixture where the curve has one, and records its absence otherwise.
- The panel is committed data with a fingerprint, so the spell baseline and homebrew runs share it. Regenerated on purpose only.
- Paired seeds (ADR 0007).
- Tests: the transform removes only level-L spells and keeps slots; selection respects the band; a level with no spellcasting fixture records that; the fingerprint changes when the panel does.

## Resolution

Built in engine 0.1.43. `spell_panel::control_party` removes from the caster role every ability that casts a spell whose own level is L, in any section. Its slots and the rest of its list stay. `bench baseline panel` commits `data/baseline/spell-panel.json`, with one panel per odd party level from 1 to 17, selected with the control for spell level (P+1)/2. Each shape keeps up to three fixtures with a control win rate from 40 to 80%, nearest 60% first. The group shape is four copies of a fixture the control beats more than 80% of the time alone, with a CR below every single-shape fixture's. The single shape swaps in an in-band spellcaster when none of its picks casts, or records `spellcasting_fixture: null`, which happens at levels 1 and 5. Each member stores its master seed, so a spell run pairs fight by fight with the control (ADR 0007). The fingerprint hashes the members, their shapes, copies and seeds. The file also records the input fingerprints and an effects-file hash, and `bench baseline status` reports when any of them is stale. A cantrip or upcast row fights the same fixtures against its own control, so the band holds only for the headline control; ADR 0020 records this and the lift runner decides how to report it. Decision report: [012 decisions](../reports/012-decisions.md).
