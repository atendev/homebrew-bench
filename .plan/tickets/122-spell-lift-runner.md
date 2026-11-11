---
id: 122
title: Spell lift runner
type: task
mode: AFK
status: open
blocked_by: [115, 120, 121]
claimed: false
---

## Question

Build spell lift from [Spell value metric](../closed/001-spell-value-metric.md) ([ADR 0020](../../docs/adr/0020-spell-lift-as-the-spell-value-metric.md)): swap a spell onto the caster role, fight the panel against the control, and produce lift rows.

## Notes

- Skills: `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- Swap: the caster role's level-L spells are replaced by the tested spell on the stat-block section matching its `casting_time`. Every spell goes on the caster role, so all share one DC and attack bonus.
- Rows: headline at party level 2L-1 cast at L; one row per upcast level N at 2N-1 for a spell with an upcast rule; cantrips at 1, 5, 11 and 17. Each row uses its own level's panel from [Control party and spell panel](121-control-party-and-spell-panel.md).
- Lift: win-rate and casualty-rate change against the control, per panel fight, with mean lift and single-monster and group columns. Each row also records damage or healing per cast as a diagnostic.
- "Never cast" when the policy never casts it in a row; "not measured" with a reason for a `long` spell, an unrunnable upcast clause, or a counter spell on a panel with no spellcasting fixture. Never zero.
- CLI: `bench batch --test-spell <id> --spell <file>`, reusing batch's assumptions, seeds, sample replay and report path.
- Paired seeds (ADR 0007) are fixed. This ticket picks the fight count and the interval method and records both in ADR 0020. The interval must be one the report can state in one plain sentence, because [Spell lift report](126-spell-lift-report.md) prints "level with" from it.
- Tests: swap keeps the rest of the list; row set per spell kind; never cast; each not-measured reason; paired seeds give identical control fights across spells.

## Resolution

<!-- Filled in at close. -->
