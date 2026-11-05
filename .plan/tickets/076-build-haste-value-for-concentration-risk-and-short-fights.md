---
id: 076
title: Build Haste value for concentration risk and short fights
type: task
mode: AFK
status: open
blocked_by: [073]
claimed: false
---

## Question

Build what [Should Haste's batch value count concentration risk and short fights](../closed/075-should-haste-value-count-concentration-risk-and-short-fights.md) decided. In `bench-core/src/tactics.rs`, a timed modifier's value counts the rounds the fight has left, the chance the caster keeps concentration through them, and Haste's lethargy as a lost bearer turn. `POLICY_VERSION` stays 3. Batch numbers change, so bump the workspace version one patch. Don't regenerate `data/baseline/curve.json`.

## Notes

Consult `tdd`, `codebase-design`, `domain-modeling` and `code-review`. The formula, the measured cells and the worked troll example are in 075's Resolution. A prototype of it gave those numbers, so match them.

Why after 073: both tickets edit scoring in `tactics.rs`. 073 makes the policy's save chance (`p_success`, inline in `effect_value`) read the engine's sources, and this ticket needs the same chance for the caster's Constitution save. 073 names no helper for it. 073 also rewrites the batch report's method sentence about teleports, and this ticket rewrites the buff half of it. Landing after 073 lets this ticket reuse 073's save chance and edit that sentence once. Unless 073 already factored it out, this ticket factors that chance into one helper (`save_success(saver, ability, dc) -> f64`, reading the stat block save plus `engine::save_modifier`, `engine::save_disposition` and `engine::automatic_save_failure`) and use it in both places.

Policy (`bench-core/src/tactics.rs`):

- Add `damage_threat(c) -> f64`. It is `threat` without the healing term: the best ready action's expected damage, each foe's share capped at its `hit_points`, averaged over living foes, and 0 when `c` can't act. Run it through `nested` like `threat` so the `MAX_THREAT_NESTING` guard holds. A clean way is to have `threat_unnested` return damage and healing apart and let both functions read it. Don't change what `threat` returns.
- Add `rounds_left(actor) -> f64`. It sums `hit_points` over the actor's `Active` opponents and divides by the sum of `damage_threat` over the actor's allies (actor included) that `can_act`. When that sum is 0, it returns `f64::from(BUFF_HORIZON)`.
- Add `concentration_keep(caster) -> f64`, the chance the caster's concentration survives one round of opponent turns:
  - For each opponent that `can_act`, first decide whether it is focused: the caster is the first entry in its `ranked_opponents(foe, None)`.
  - For each of its ready actions (as `incoming_rolls` filters them), take the counted damage on the caster. A focused opponent counts the action's attack-roll damage (`roll_value` with `Rolls::Attacks(AttackFilter::Any)`) and all its save damage (`Rolls::Saves(None)`). An opponent that isn't focused counts only save damage from `Effect::Area` effects, found through casts and `use_action` the way `roll_value` walks them. A single-target `SavingThrow` is aimed like an attack, so it counts only when focused. Add a small walk (or a `Rolls` variant) that returns area save damage on its own.
  - Pick the action with the most counted damage, ties to the earlier action. Choosing by counted damage matters: in the seed-7 replay of fight 1600 (adult red dragon, level 17) the dragon's Multiattack (51.5) outscores its breath (43.1) on the caster. Picking by total damage and then dropping the uncounted attacks would ignore a ready breath whenever the dragon ranks someone else first.
  - When the counted attack and save damage, summed over opponents, is at least `hit_points(caster)`, return 0.
  - Otherwise multiply, for each opponent and its picked action: `(1 - p_fail(max(10, floor(a / 2))))^(d / a)` for its counted attack damage `d` with average hit `a` (the second value `roll_value` returns), skipped when either is 0; and `1 - p_fail(max(10, floor(s / 2)))` for its counted save damage `s`, skipped when 0.
  - `p_fail(dc)` is `1 - save_success(caster, Con, dc)`, the helper above, which carries Advantage and Disadvantage.
- Add `acts_before_opponents(actor, bearer) -> bool`. Walk `state.initiative_order` from the entry after the actor, wrapping. Return true if the bearer comes up before any opponent of the actor that `can_act`, and false otherwise. Always return false when the bearer is the actor or the actor isn't in the order.
- `bearer_value` takes the actor as a new first argument (both callers in `plan_for` and `modifier_plan` pass `actor`). Compute `rounds_left(actor)` once, and `concentration_keep(actor)` lazily, once, the first time a modifier needs it. For each modifier:
  - `cap = horizon(&m.duration)`. Skip the modifier as today when it is held or `cap` is 0.
  - `h = min(cap, rounds_left)`.
  - `k = concentration_keep(actor)` when `m.duration` is `Duration::Concentration`; otherwise 1.
  - `f = 1` when `acts_before_opponents(actor, bearer)`, else 0.
  - `rounds = Σ_{i=1..ceil(h)} k^(i - f) × clamp(h - (i - 1), 0, 1)`. Replace every use of the old `rounds` (AC, save Advantage, attack bonus, extra action) with it. With `k = 1` and `h = 2` this is 2, as now.
  - Lethargy: when `m.duration` is `Concentration`, `m.on_end` holds an `Effect::Condition` whose condition is one of `can_act`'s list (Incapacitated, Paralyzed, Petrified, Stunned, Unconscious), and `can_act(bearer)`, subtract `threat(bearer) × (1 - k^h)` from `out.damage`. Count it once per modifier that carries such an `on_end`. Keep the condition list in one place shared with `can_act`, not a second copy.
  - A plan's score can go below 0 this way. Leave it. The policy still takes the best-scoring plan, as today.
- The plan reason gains the new terms after the existing ones, only when they differ from the neutral value. Add `{rounds:.1} of {h:.1} rounds at keep {k:.2}` when `k < 1` or `h < cap`, and `lethargy -{cost:.1}` when the cost is above 0. For example: `haste on standard-striker-05 (+2 AC against 20.8 attack damage, advantage on dex saves against 0.0 dex save damage, extra action 15.0, 1.3 of 1.8 rounds at keep 0.34, lethargy -12.9)`.
- Update the doc comments on `BUFF_HORIZON` and `bearer_value` to say the horizon is cut to the rounds left and weighted by the keep chance. No new public constant.

Docs:

- ADR 0012, the buffs paragraph (amended by 056 and 073): replace "valued over `BUFF_HORIZON` (2) rounds: its rounds when shorter, the whole horizon for concentration and longer durations" with the rounds-left cap and keep-chance weighting from 075, and replace "and Haste's lethargy is not scored" with the lethargy cost. Name 075 in the paragraph's ticket list.
- The batch report's method sentence in `bench-core/src/batch.rs`: "over two rounds on the ally it helps most" becomes "over up to two rounds, no more than the fight is expected to last, weighted by the chance the caster keeps concentration, on the ally it helps most", and "Haste's extra action as one more attack" becomes "Haste's extra action as one more attack less a lost turn when lethargy is likely". Update the test that checks the sentence.
- `CHANGELOG.md` under Changed: the batch policy now values a buff by the rounds the fight has left and the chance the caster keeps concentration, and counts Haste's lethargy as a lost turn. The standard caster casts Haste far less, almost never against monsters the party beats in one or two rounds, and batch numbers change.
- `CONTEXT.md` already has **Buff horizon** (added by 075). Check it still matches the code.

Tests (`tdd`). Build the tactics tests on small states, like the existing `caster_with` tests, and give expected values as formulas over the state's numbers rather than magic literals:

- `rounds_left`: two foes of 30 and 20 HP against allies whose `damage_threat` sums to 25 give 2.0. A healer's healing doesn't count. Allies with no damage give `BUFF_HORIZON`.
- `concentration_keep`:
  - A foe whose top-priority target is another ally and that has no save effects gives 1.
  - The same foe, once the caster is its top target, gives `(1 - p_fail(10))^(d / a)`. For Con +2 that is `0.65^(d / a)`.
  - A foe whose save action would deal the caster 53 gives DC 26 and 0 for Con +2.
  - Counted damage at or above the caster's HP gives 0.
  - Action choice by counted damage: a foe that ranks the striker first, with a ready area breath and a Multiattack that outscores the breath on the caster, gives the breath's area-save term (DC 26 and 0 for a 53-damage breath at Con +2), not 1.
  - A foe that ranks the striker first and whose only damage is a single-target save blast leaves the caster's keep chance at 1. The same blast with the caster ranked first counts.
  - Advantage on Constitution saves from a timed modifier raises it to `(1 - p_fail²)^(d / a)`.
- `acts_before_opponents`: true for order caster, striker, foe; false for caster, foe, striker; false for the caster itself; skips a foe that can't act.
- Weighted rounds:
  - With `k = 1` and a long fight, Haste on the striker scores what it scores today (keep a copy of `the_caster_hastes_the_striker_over_itself_once_the_extra_attack_counts` on a state where no foe targets the caster and `rounds_left >= 2`).
  - With `rounds_left` 0.5, the extra-action value is half of today's.
  - With the striker between caster and foe in initiative, the first round counts at `k^0`.
- Lethargy:
  - A foe that ranks the caster first and breaks concentration often gives a reason with `lethargy -` and a cost of `threat(striker) × (1 - k^h)`.
  - The same Haste without the extra-action modifier's `on_end` has no lethargy term.
  - A Shield-like `self` +5 AC for `rounds(1)` has no keep or lethargy term and only the rounds-left cap.
- Choice:
  - Against the ogre stat block with the level 11 party (`rounds_left < 1`), the caster's first plan isn't Haste.
  - Against a high-HP stat block that ranks the defender first, does no save damage and lasts more than two rounds, the caster still hastes the striker.
- Existing Haste tests in `tactics.rs` (`the_caster_hastes_the_ally_with_the_highest_incoming_damage`, the ties test, `the_caster_hastes_the_striker_over_itself...`) keep their assertions. Where the new value turns them, change the state (more foe HP, a foe that targets someone else), not the expected bearer.
- Batch tests in `batch.rs` that need Haste cast against the ogre at level 5 (`a_hasted_striker_makes_one_extra_attack_per_turn_and_never_a_multiattack`, `when_haste_ends_the_striker_loses_its_next_turn` and the broken-attacks warning test) will see no Haste once this lands. Move them to an in-test stat block that is long-lived and doesn't rank the caster first, as above, so they still exercise the extra-action section and lethargy. Keep what they assert.

Accepted approximations (075), not to fix here: legendary actions and Opportunity Attacks are left out of the keep chance; `f = 1` ignores a legendary window between the actor's turn and the bearer's; and `d / a` is a fixed hit count used as an exponent instead of a per-attack distribution.

Measure before closing. Run release builds of master and the branch from `git archive` in a temp dir with `HOME=$(mktemp -d)`, never the real `~/.bench`, using `bench batch <id> --levels <L> --fights 100 --seed 7`. Report wins, PC deaths, monster DPR mean and rounds mean for ogre, knight and adult red dragon at levels 5, 11 and 17. Count Haste casts from `--fight` replays of the dragon cells at 11 and 17. 075's prototype gave dragon L11 44 wins and 47 casts, L17 99 wins and 6 casts, and ogre and knight equal to pre-056 with no casts. If the build differs by more than seed noise, explain why in the Resolution.

Versions and baseline:

- `POLICY_VERSION` stays 3. Bump the workspace version, and with it `ENGINE_VERSION`, one patch from what master holds when this lands (0.1.13 if 073 ships 0.1.12). No live-state field changes.
- Don't regenerate the curve. If `a_fixture_regenerates_to_its_committed_row` differs and is not ignored yet, add `#[ignore = "curve stale until ticket 074 regenerates it"]`. The stale reasons in `stale_beyond_the_engine` and the CLI stale-curve test already cover a policy and engine change.
- Ticket 074 waits on this ticket so its one regeneration covers it.

## Resolution
