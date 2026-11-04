---
id: 063
title: Build batch reactions policy
type: task
mode: AFK
status: open
blocked_by: [046, 062]
claimed: true
---

## Question

Build the batch reaction policy decided in [Batch policy takes reactions](../closed/046-batch-policy-takes-reactions.md). Batch encounters open reaction windows, and the tactics policy decides each one. It takes a hit reaction (Shield, Parry) only when it turns the hit into a miss. It takes an Opportunity Attack when the attack is worth more than the reaction reserve. The step-out penalty prices the attacks the policy will really take. `POLICY_VERSION` moves from 2 to 3. The baseline is not regenerated here; ticket 056 does that once.

## Notes

Consult `tdd`, `codebase-design`, `domain-modeling` and `code-review`. Read ADRs 0012, 0015 and 0018 (written by ticket 062) first, and the decider seam ticket 062 built. Don't change how live mode decides windows.

Engine seam (`bench-core/src/engine.rs`):
- The policy needs, for each window: the reactor, the window kind, the options on offer, and the mover or attacker. A hit window also needs the attack total, the target's AC before the reaction, and whether the d20 was a natural 20. If 062's window description lacks any of these, add them. This adds fields only. Live behavior and the stored state must not change, and `ENGINE_VERSION` does not move for it.
- Expose the shared Opportunity Attack eligibility rule (`opportunity_eligibility`, or 062's factored "can react" check plus the provoke checks) as a crate-visible function the policy can call. Tactics must not copy the rule.

Policy (`bench-core/src/tactics.rs`):
- `POLICY_VERSION = 3`.
- A named constant for the value of one AC point against one round of incoming damage, `0.1`. Ticket 056 reuses it for `TimedModifier` AC buffs, so put it where both can read it.
- `ac_bonus(block, option)`: the AC bonus n a hit-trigger reaction gives, read from its `timed_modifier` on `armor_class` (through `bound_cast_effects` for a cast such as Shield). Options with no AC bonus have n = 0 and are never taken.
- `reaction_reserve(reactor)`: for the reactor's best ready hit-trigger reaction ("ready" means `ability_ready` and an open slot), `min(0.1 × n × D, H)`, else 0. Don't use `incoming()`: it sums `threat()`, which counts save damage and healing.
  - D is the expected damage from attack rolls the reactor's active opponents deal it: for each opponent, its best ready action counting only `Attack` effects (walking Multiattack steps; saves, areas, heals and non-attack casts add nothing). A reaction whose trigger filter is `melee` counts only attacks with `range: None`; `ranged` only ranged ones.
  - H is the largest average damage on a hit (dice average plus modifier, after the reactor's resistances) of any one matching attack among those opponents.
- `opportunity_value(reactor, mover)`: the highest expected damage among the reactor's at-will single melee attacks (the same leaves `opportunity_attack_action` walks, cast steps skipped) against the mover, capped at the mover's hit points, plus `threat(mover)` when the damage would drop it. Return the chosen attack id with it. Ties keep stat-block order.
- A public decider entry point the batch runner installs:
  - Hit window: among options with total < AC + n and no natural 20, take the one with no usage limit or slot first, then the lowest slot, then stat-block order. Otherwise pass.
  - Opportunity Attack window: take with the chosen attack id when `opportunity_value > reaction_reserve`, strictly. Otherwise pass.
  - The decision reads the state as the engine presents it at that window. No look-ahead.
- Step-out penalty in `plan_for` (around line 470): replace `best_single_attack` with the sum of `opportunity_value` over engaged opponents that pass the engine's eligibility rule and that the decider would attack with. Keep the reason text. Ticket 056's teleport formula also prices avoided attacks, and it will call the same penalty function, so make it one function that both can call. Delete `best_single_attack` once nothing uses it.
- Recursion guard: `opportunity_value`, `reaction_reserve` and the penalty function each take a level of `threat_nesting`, the way `threat()` does, and return 0 at or past `MAX_THREAT_NESTING`. Ticket 056's teleport score calls the penalty, which reaches the reserve and `threat()`, then `ability_value`, and could come back to the teleport score.
- Log a reason line for every decided window by returning it in `Decision::Answer { reason }`, which the engine logs as a `reaction_reason` event, in the shape of plan reasons, such as `goblin-warrior: Opportunity Attack on standard-caster-03 (scimitar, expected 4.2 > reserve 0.0)` or `standard-caster-05: Shield (17 vs AC 13+5 turns the hit into a miss)`. Passed windows log one line too, so a sampled log shows why a reaction was held.

Batch (`bench-core/src/batch.rs`):
- Pass `reaction_windows: true` in batch's `StartSpec` (it sets `settings.reaction_windows`) and install the policy decider with the run's `PolicyConstants`. 062's decider is `ReactionDecider` in `bench-core/src/reaction.rs`, an enum on `state.reactions`; add a policy variant beside `PassAll`.
- Keep passing an empty `opportunity` list on moves. The windows fill it.
- The report's method sentence: replace "takes no reactions" with what v3 does, in one sentence. Ticket 056 rewrites the buff half of the same sentence.

Tests (write them first):
- Hit window: an ogre's hit on the level 5 caster with total in [AC, AC + 5) takes Shield and misses, spending the slot and the reaction. A total of AC + 5 or more passes. A natural 20 passes. No slot left passes.
- A knight's Parry takes on a melee hit inside its band and passes outside it. Parry never opens on a ranged hit (062's filter, checked again here through batch).
- Two hit options that both stop the hit: the one with no limit wins over the slot.
- Opportunity Attack: a goblin with no hit reaction attacks a caster that steps out. A reactor whose reserve is above its attack value passes, and the same reactor with no slot left attacks. The chosen attack is the higher-damage melee leaf, not the first one.
- An attack that would drop the mover adds the mover's threat to its value.
- Penalty: a ranged step-out away from a Disengaged reactor, a no-provoke mover, a Blinded reactor and a reactor that would pass all price 0. A reactor with only a ranged attack prices 0. The penalty equals the summed `opportunity_value` of the reactors the decider would attack with.
- Reserve: an opponent that deals only save damage (a breath weapon, no attack rolls) leaves a reserve of 0, so the reactor takes the Opportunity Attack. A healer opponent adds nothing. Parry's reserve counts melee attacks only. A reserve never exceeds the largest single matching hit: a Knight with Parry facing the level 11 party takes an Opportunity Attack whenever its attack value is above the largest single party hit.
- Recursion: a stat block with a teleport action and a ranged attack, engaged with a reactor that has Parry, scores a step-out plan and returns. At `MAX_THREAT_NESTING` the penalty and the reserve are 0.
- Determinism: the same seed gives the same batch log twice. `fingerprint` includes version 3.
- Batch: a fixed-seed batch on goblin-warrior, the knight and one adult dragon at levels 1, 5 and 11 against `master`. Numbers are expected to move. Record each shift in the Resolution, and explain any shift you can't trace to a logged reaction.

Docs:
- Amend ADR 0012: a reactions paragraph covering the hit flip rule, the Opportunity Attack against the reserve, the shared penalty, decisions one window at a time, and no Disengage plans. Name the 0.1 constant.
- Amend ADR 0015's consequence that batch is unchanged until ticket 046, and ADR 0018's decider seam bullet, to point at policy v3.
- `.agents/skills/homebrew-bench/references/batch-mode.md`: batch now takes Opportunity Attacks, Shield and Parry. Riposte's extra attack stays an agent clause.
- `CHANGELOG.md` under Unreleased: policy v3 takes reactions, the penalty fix, and the curve reading stale until it is regenerated.

Version and baseline:
- `POLICY_VERSION` goes to 3 here and nowhere else. Don't bump `ENGINE_VERSION` for the window fields.
- Don't regenerate `data/baseline/curve.json`. Add the policy stale reason ("the tactics policy is ... v3 and the curve used ... v2") to the allowances in `stale_beyond_the_engine` (`bench-core/tests/calibration.rs`) and in `batch_compares_against_the_embedded_curve_outside_the_repo` (`bench/tests/cli.rs`).
- `a_fixture_regenerates_to_its_committed_row` regenerates goblin-warrior and compares it with the committed row. Goblins now make and take Opportunity Attacks and the caster casts Shield, so the row will differ. If the row differs and the test is not ignored yet, add `#[ignore = "curve stale until ticket 056 regenerates it"]`. Ticket 059 or 061 may have added it already. Don't edit the committed row.
- `spellcasting_fixtures_keep_their_difficulty_bands` may shift. If a band moves, find the reaction behind it in the log before widening the expected set, and record it.

Hand-off to 056: 056 reuses the AC point constant for `TimedModifier` and the penalty function for teleports, keeps `POLICY_VERSION = 3` and the recursion guard, regenerates the curve once, removes all four stale allowances (engine, fixture set, party, policy) and removes the ignore on `a_fixture_regenerates_to_its_committed_row`, whichever ticket added it.

Overlaps: `tactics.rs` is also touched by 056 (scoring) and 061 (`disposition` in the engine; 056 fixes the policy copy). `batch.rs`'s report sentence is shared with 056. Whichever lands second rebases.

## Resolution
