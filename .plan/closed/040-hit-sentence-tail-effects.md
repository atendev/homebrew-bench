---
id: 040
title: Hit-sentence tail effects
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

The importer keeps only later sentences of an attack's Hit clause as agent clauses. Effects stated in the Hit sentence after the damage are dropped without a trace: Clay Golem's Hit Point maximum loss, Otyugh's Poisoned condition, and others. Capture the tail of the Hit sentence as an agent clause, the way the saving-throw path already keeps its remainder, and check which fixtures change.

## Notes

Consult `implement`, `tdd` and `domain-modeling`. Raised by the ticket 037 audit; see [closed/037](../closed/037-conditional-and-later-clause-damage.md). The tail may start with "and" or ", and" once the damage phrases are removed, so decide how to trim it. Rerun `python3 -I scripts/import-srd-monsters.py` and `cargo test --all` afterward and report which fixtures gained clauses.

## Resolution

`hit_tail` in `scripts/import-srd-monsters.py` now keeps what a Hit sentence states after its damage as an agent clause. It strips every dice or flat damage phrase (unicode minus included) and keeps a tail only when it opens with "and", trimming the "and", capitalising and ending with a period. A sentence with no damage at all (Roper's Tentacle) is kept whole. Tails that open with "or", "plus" or "if" are conditional damage that `place_damage` already keeps, so they are not repeated. The clause goes after any conditional-damage clauses and before later-sentence clauses, on every branch of `attack`, including the flat-damage one.

Result: 30 fixtures gained a clause and moved from `modeled` to `partly_modeled`: assassin, balor, bearded-devil, bone-devil, clay-golem, couatl, darkmantle, ettercap, ettin, fire-giant, frost-giant, giant-centipede, giant-vulture, hill-giant, homunculus, incubus, lich, merrow, oni, otyugh, pirate-captain, quasit, roper, shadow, sprite, stirge, storm-giant, tarrasque, worg, wyvern. The graduated count stays 305 of 341. Two new tests in `bench-core/tests/fixtures.rs`; `cargo test --all` passes. Audit: APPROVE on round 2 (round 1 found the flat-damage branch, fixed).

Known gaps, none in the SRD today: a tail after conditional damage ("..., and the target ...") is dropped, and a non-"and" effect after damage is dropped. Half-dragon's "plus 7 (2d6) damage of the type chosen" is still unmodeled damage, outside this ticket.
