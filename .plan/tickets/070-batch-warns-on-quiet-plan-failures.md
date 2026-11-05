---
id: 070
title: Batch warns when plans fail or casts are never ready
type: task
mode: AFK
status: open
blocked_by: []
claimed: true
---

## Question

Make `bench batch` report, in its summary warnings, every engine error a plan threw and every cast the policy could never use, so a fault that costs a combatant its turns can't hide in unsampled logs.

## Notes

Found by the audit of [Build upcast spell scaling](../closed/059-build-upcast-spell-scaling.md). Heal's casts failed with `invalid dice expression 70` in every fight on master before 059, and nobody saw it, because the failure only reached sampled logs.

Engine and batch (`bench-core/src/batch.rs`, the plan loop around `plan failed again`):

- When `execute` returns an error for a plan, count it by combatant name, action name and error text across the whole batch. Keep the sampled log lines as they are.
- Add one summary warning per distinct (combatant, action, error): `Ogre Mage: Cone of Cold failed 412 times (invalid dice expression 70); the turn was lost each time`. Sort them by count, highest first.
- A `Cast` on a stat block whose `slot_level` is below the spell's `level` is never ready (`ability_ready`), so the policy silently never picks it and the monster reads weaker than it is. Refuse it at stat-block validation, beside the recharge dice check, with the reason "a cast's slot level is below the spell's level", so `bench start`, `bench add` and `bench batch` all stop before any dice are drawn. Party spell lists already pass this; the check covers homebrew.
- Warnings don't change any fight, so batch numbers, the dice stream and the baseline don't move. Don't bump the engine version for warnings alone. Bump one patch from what `master` holds when this lands only if the validation change alters a committed fixture or party file (it shouldn't).

Tests (`tdd`):

- A batch whose monster has an action that always errors reports one summary warning with the right count, and the fight still finishes.
- Two different errors on one action give two warnings.
- A clean batch has no plan-failure warning.
- A homebrew stat block with a level 3 spell cast at `slot_level` 1 is refused by `bench batch` with the reason above, and by `bench start`.
- Every SRD fixture and party file still validates.

Docs: `CHANGELOG.md` under Added (the warning) and Changed (the refusal). The batch-mode skill reference lists the new warning.

Consult `tdd` and `code-review`.

## Resolution
