---
id: 012
title: Control party and spell panel
type: task
mode: AFK
status: open
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

<!-- Filled in at close. -->
