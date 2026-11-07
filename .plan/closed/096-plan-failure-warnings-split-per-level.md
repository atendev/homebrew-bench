---
id: 096
title: Plan-failure warnings split per level
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

Batch keys a plan failure on the combatant's name and the error text, so one fault in a standard-party PC repeats once per party level, and errors that name a combatant id split further. Should the key use the stat block's archetype and strip ids from the text?

## Notes

- Source: [Batch warns on quiet plan failures](../closed/070-batch-warns-on-quiet-plan-failures.md).
- Reproduce on the current engine first: run a batch with a deliberately faulty standard-party entry and count the warnings.
- Check whether the report format needs a count per warning instead of a repeat.
- Skills: `grilling`, `domain-modeling`. If the answer is code, the build is a follow-up `task` ticket.

## Resolution

Yes, as code. A party member's plan failures key on its role instead of its name, combatant ids in the error text become the same labels, and each role-keyed row lists the levels it came from. The build is [Build plan-failure warnings by role](../tickets/106-build-plan-failure-warnings-by-role.md).

Reproduced on engine 0.1.25: a copy of the standard party with the striker's Longbow dice set to `70` at levels 3-5, ogre, 20 fights per level, seed 7, gives three rows for one fault: `Standard Striker (Level 3): Longbow failed 10 times (invalid dice expression 70); the action was lost each time`, then Level 4 (6 times) and Level 5 (6 times).

- Key. A party member keys on its role (defender, striker, healer, caster), found in its stat block id with the match `group_party` already uses to order members. Both use one shared helper. When an id names two roles, the first in `ARCHETYPES` order wins. A member keys on its name instead when its id names no role, or when more than one member of that level's party matches the same role, so a `--party` with two strikers never merges two creatures. The monster keeps its name. The standard party has one member per role at every level, so its rows always merge. A name-keyed member still repeats once per level, since names usually carry the level (`Standard Striker (Level 3)`). That is accepted: those cases are rare, and a repeat is safe where a wrong merge would point the designer at the wrong creature. A user can make a member merge by putting its role word in the stat block id. Stripping ` (Level N)` from names was rejected because it guesses at a naming convention.
- Label. A role-keyed row reads with the capitalized role and the levels it failed at, as compact ascending ranges: `Striker (levels 3-5): Longbow failed 22 times (invalid dice expression 70); the action was lost each time`. One level reads `level 5`, and scattered levels read `levels 1-4, 7, 9-10`. Monster and name-keyed rows read as today, with no level list, so 070's monster rows and tests don't change: a monster has one stat block for every level, so its levels point at no file. Per-level counts stay in the logs.
- Ids in the error text. Each combatant id in the error text is replaced with that combatant's label: the bare role (`Striker`, no levels) for a role-keyed member, the name otherwise, and the monster's name for the monster. The id is resolved through the fight's combatant list, not parsed. Matching runs longest id first on whole tokens only (the characters before and after are each the text's edge or not a letter, digit or `-`), so `standard-striker-05-1` never matches inside `standard-striker-05-10`. `standard-striker-03-1 cannot act` and `standard-striker-04-1 cannot act` become one row, `Striker cannot act`.
- Same text, different fault. Role-keyed rows with identical error text at different levels merge, and the level list shows where each happened. Different error texts stay separate rows.
- Count. The ticket asked whether the report needs a count per warning. It already has one (`failed <n> times`). Merging makes that count cover the whole fault, so no new field is needed.
- Unchanged. The `plan failed` log lines (they format the engine error directly, not the rewritten text), live-mode error text, batch numbers and the dice stream don't change. No fixture or party file changes, so the engine stays 0.1.25, the policy stays v4, and the curve isn't regenerated. A CHANGELOG line goes with the build.
- Vocabulary. The ticket said "archetype", but `CONTEXT.md` defines the four slots as roles, so the warning text and docs say "role". The `ARCHETYPES` constant keeps its name. No `CONTEXT.md` change and no ADR, since the change is reversible and has no real trade-off.
