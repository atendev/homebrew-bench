---
id: 070
title: Batch warns when plans fail or casts are never ready
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
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

Each fight's `Log` now records a `PlanFailure` (combatant name, action name, error text, and whether the turn was lost) whenever `execute` refuses a plan: the first try that gets recomputed, the retry that loses the action, and a failed legendary action. `FightResult.plan_failures` carries them (skipped in JSON), and `run` adds one summary warning per distinct (combatant, action, error), most frequent first: `Ogre: Multiattack failed 50 times (invalid dice expression 71); 25 turns were lost` (from the two-error test, levels 1-3 at 4 fights). Sampled log lines are unchanged.

- Decision: count every refusal, not only lost turns, and say the cost in the tail (`the turn was lost each time`, `<n> turns were lost`, or `a recomputed plan ran instead each time`). A deterministic fault fails the first try and the retry, so it counts two failures per lost turn; the tail keeps that honest. One failure reads `failed once`.
- Stat-block validation. `checked_entries` in `stat_block.rs` now refuses a cast whose `slot_level` is below its spell's level, top-level `cast` effects and Multiattack step casts alike, with `<ability-id>: a cast's slot level is below the spell's level (<spell> is level <n>, cast with a level <slot> slot)`. Every CLI path loads stat blocks through `read_stat_block`, so `bench start`, `bench add`, `bench batch` and `bench schema validate` stop before any dice. A spell missing from the effects file is left to the missing-spell warning.
- Refactor: the effects-file cache moved from `engine::load_spell_effects` to `spell::catalog`, so validation and the engine share one parse.
- Tests. `an_action_that_always_errors_warns_once_with_its_count` checks the count against the `plan failed` lines in every replayed fight. `two_errors_on_one_action_give_two_warnings` uses a Multiattack over two broken attacks. `a_clean_batch_has_no_plan_failure_warning` covers the ogre. The CLI test `a_cast_below_its_spell_level_is_refused_before_any_dice` covers `bench start` and `bench batch`. The existing fixture and standard-party tests still pass.
- Numbers. No fixture or party file changed, so the engine stays 0.1.10. Release builds of master and this branch gave byte-identical stdout, report and logs for ogre and adult red dragon at seed 7, 40 fights per level. The curve wasn't regenerated. `cargo test --workspace` passes, clippy shows only master's two engine.rs errors, the SRD import exits 0 with 341 of 341 graduated, and `check-spell-effects.py` passes.
- Fog: a cantrip cast with a `slot_level` is also never ready and isn't refused. And failure keys use combatant names and error text, so one standard-party fault splits across levels (each level's PC has its own name) and across error texts that name a combatant id.

