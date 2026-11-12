---
id: 013
title: Spell lift runner
type: task
mode: AFK
status: closed
blocked_by: [006, 011, 012]
claimed: false
---

## Question

Build spell lift from [Spell value metric](../closed/001-spell-value-metric.md) ([ADR 0020](../../docs/adr/0020-spell-lift-as-the-spell-value-metric.md)): swap a spell onto the caster role, fight the panel against the control, and produce lift rows.

## Notes

- Skills: `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- Swap: the caster role's level-L spells are replaced by the tested spell on the stat-block section matching its `casting_time`. Every spell goes on the caster role, so all share one DC and attack bonus.
- Rows: headline at party level 2L-1 cast at L; one row per upcast level N at 2N-1 for a spell with an upcast rule; cantrips at 1, 5, 11 and 17. Each row uses its own level's panel from [Control party and spell panel](012-control-party-and-spell-panel.md).
- Lift: win-rate and casualty-rate change against the control, per panel fight, with mean lift and single-monster and group columns. Each row also records damage or healing per cast as a diagnostic.
- "Never cast" when the policy never casts it in a row; "not measured" with a reason for a `long` spell, an unrunnable upcast clause, or a counter spell on a panel with no spellcasting fixture. Never zero.
- CLI: `bench batch --test-spell <id> --spell <file>`, reusing batch's assumptions, seeds, sample replay and report path.
- Paired seeds (ADR 0007) are fixed. This ticket picks the fight count and the interval method and records both in ADR 0020. The interval must be one the report can state in one plain sentence, because [Spell lift report](017-spell-lift-report.md) prints "level with" from it.
- From [Control party and spell panel](../closed/012-control-party-and-spell-panel.md): build each row's control with `spell_panel::control_party(party, row_level, book)` and fight each member with its stored `seed` and `copies`. The panel's band holds only for the headline control, (P+1)/2. A cantrip or upcast row's control may win more or less often, so decide whether the row reports its own control win rate.
- Tests: swap keeps the rest of the list; row set per spell kind; never cast; each not-measured reason; paired seeds give identical control fights across spells.

## Resolution

Built in engine 0.1.44. `bench batch --test-spell <id> [--spell <file>]` swaps the spell onto the caster role (replacing any ability that casts it, in the section its `casting_time` names, at the row's slot level) and fights every panel member with the row's own control and with the swapped party on the member's stored seed and copies. Rows: headline at 2L-1, one per upcast level N at 2N-1 for a spell with an upcast rule, four cantrip rows at 1, 5, 11 and 17. Each measured row gives the mean change in win rate and in fights with a party death over every paired fight with a 95% interval (mean ± 1.96 standard errors of the per-fight differences, 100 fights per member, recorded in ADR 0020), single and group columns, the row's pooled control win rate and whether it sits in the 40 to 80% band, and damage and healing per cast as a diagnostic. A stalemate counts as not won. `never_cast` and `not_measured` (with a reason: `long` spell, an SRD upcast clause the engine doesn't apply, a counter spell with no spellcasting fixture) carry no numbers. Output is stdout JSON plus `lift.json` and sample logs under `reports/<id>-lift/`; `--fight --row --member` replays a sample. `--seed` and `--levels` are refused; `--threads` defaults to 4. Fights now tally the party's casts per spell. Decision report: [013 decisions](../reports/013-decisions.md).
