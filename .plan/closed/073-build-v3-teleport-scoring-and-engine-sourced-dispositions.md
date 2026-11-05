---
id: 073
title: Build v3 teleport scoring and engine-sourced policy dispositions
type: task
mode: AFK
status: closed
blocked_by: [056]
claimed: false
---

## Question

Split from [Build v3 scoring for buffs and forced movement](056-build-v3-scoring-for-buffs-forced-movement-and-teleports.md) for size. In `bench-core/src/tactics.rs`, score teleports by the Opportunity Attacks they avoid, and make the policy read attack and save dispositions from the engine instead of its own partial lists. Amend ADR 0012 where these change it. `POLICY_VERSION` stays 3. Batch numbers change, so bump the workspace version one patch. Don't regenerate `data/baseline/curve.json`. If `a_fixture_regenerates_to_its_committed_row` differs, add `#[ignore = "curve stale until ticket 074 regenerates it"]`.

## Notes

Consult `tdd`, `codebase-design`, `domain-modeling`.

- Teleports score the Opportunity Attacks they avoid by calling 063's penalty function (`Ctx::step_out_penalty`): eligible engaged reactors the policy would attack with, each at its `opportunity_value`. This replaces ticket 045's "sum of `best_single_attack` over engaged foes that still have their reaction", because 046 decided the mover prices only the attacks the policy really takes. The penalty and the reserve return 0 at or past `MAX_THREAT_NESTING` (063 builds that guard), because the teleport score calls the penalty, which reaches `threat()` and `ability_value` and can come back to a teleport. Keep the guard, and add a test: a stat block whose actions include a teleport and a Multiattack that uses it, engaged with a reactor that has Parry, gets a plan without overflowing the stack, as the Dryad test does for control.
- Fix `disposition` in `tactics.rs`: it sums and clamps, so an attacker with two Advantage sources and one Disadvantage source scores as Advantage. Use the two-flag rule from ticket 061 (both or neither is straight), with a test that two of one and one of the other score straight.
- The policy's `disposition` must also read the sources [Align attack-roll condition sources](067-align-attack-roll-condition-sources-with-2024-rules.md) added (ADR 0017): Prone by engagement, Grappled and Frightened by `source`, Invisible through the sight rule, downed PCs as Unconscious. Call the engine's `disposition` (ticket [068](068-build-attack-roll-condition-sources.md) makes it `pub(crate)`) instead of keeping a second list, reading engagement as the plan leaves it when the plan moves, and test that a ranged plan against a Prone target scores Disadvantage.
- The policy's saving-throw value (`p_success` in `effect_value`) reads no save sources: it misses the automatic Strength and Dexterity failures, Restrained's Dexterity Disadvantage, lighting and timed save modifiers (Haste). Ticket [069](069-build-saving-throw-condition-sources.md) makes the engine's `automatic_save_failure`, `save_disposition` and `save_modifier` `pub(crate)`; call them instead of a second list, and test that a Dexterity save against a Paralyzed target scores as a certain failure.

## Resolution

Built on branch `ticket-073-build-v3-teleport-scoring-and-engine-sourced-dispositions`, test-first.

- Engine. `engine::disposition_with(state, actor, target, within, extra)` (`pub(crate)`) is `disposition` with "within 5 feet" given by the caller and extra sources combined by the same two-flag rule. `disposition` now calls it with `within_5_feet` and no extras, so engine behavior is unchanged.
- Attack dispositions. The policy's own condition list and its sum-and-clamp `disposition` are gone. `effect_value` takes a `Footing` (replacing the `ranged_in_melee` flag): how the plan moves the actor (`Stays`, `In` to engage the target, `Out` of every engagement) and whether the policy's close-combat Disadvantage applies. "Within 5 feet" is read from it (engagement as held, true after a move in, false after a step-out), and the close-combat Disadvantage is passed as an extra source, so two Advantage sources and one Disadvantage source score straight.
- Saves. `Ctx::save_success(saver, ability, dc) -> f64` is the one save-chance helper ticket 076 asked for: 0 when `engine::automatic_save_failure` applies, otherwise the stat block save plus `engine::save_modifier`, with `engine::save_disposition` giving `1 - (1 - p)²` or `p²`. `effect_value`'s saving throw calls it.
- Teleports. `Effect::Teleport` scores `Ctx::teleport_value`: `step_out_penalty` over the active opponents engaged with the actor, 0 when there are none or the plan steps out. It lands in a new `Value::survival`, which the plan's survival and Multiattack choice ranking (`Value::worth`) read, and the reason adds `teleports clear of X opportunity damage`. The `MAX_THREAT_NESTING` guard in `nested` stops the teleport → penalty → `threat` → teleport loop.
- Tests (tactics): two Advantage and one Disadvantage score straight (3.375 for +5 against AC 12 with 1d8); a bow against a Prone target scores Disadvantage (3.4475) and a knife that moves in scores Advantage (4.53375); a Dexterity save by a Paralyzed target is a certain failure (33 of 6d10) while a Constitution save keeps its 0.3; Restrained gives Dexterity saves Disadvantage (0.09); a teleport scores the brute's Opportunity Attack value and the plan reason names it; a teleport avoids nothing when the reactor would hold for Parry or nothing is engaged; two engaged stat blocks with a teleport, a Multiattack using it and Parry each get a plan without overflowing.
- Docs. ADR 0012 gains a "Teleports and roll sources" paragraph. The batch report's method sentence now values "a teleport as the Opportunity Attacks it avoids" and says the policy "reads Advantage, Disadvantage and automatic save failures from the same sources the engine rolls with"; its test checks it. `CHANGELOG.md` has the Changed entry. `CONTEXT.md` needed no new term.
- Versions. `POLICY_VERSION` stays 3. Workspace and `ENGINE_VERSION` are 0.1.12. No live-state field changed. The curve was not regenerated. `a_fixture_regenerates_to_its_committed_row` still passes, so it isn't ignored; the stale-curve tests pass. `cargo test --workspace` passes, clippy shows only master's two engine.rs errors, and the SRD import exits 0 with 341 of 341 graduated.
- Cell shift. Release builds of master and this branch from `git archive` in `/tmp`, run with `HOME=$(mktemp -d)`, `bench batch <id> --levels <L> --fights 100 --seed 7`. Wins, PC deaths, monster DPR mean and rounds mean, master to branch:
  - adult red dragon L11 35 wins, 65 deaths, DPR 101.78, rounds 4.95, and L17 92 wins, 8 deaths, DPR 108.68, rounds 4.34: identical.
  - ogre L5, mage L5 and L11, archmage L17: identical. knight L5 DPR 9.13 to 9.16, rounds 2.46 to 2.44. archmage L11 DPR 54.53 to 54.37. Wins and deaths unchanged.
  - No SRD fixture models a teleport (the mage's and archmage's Misty Step and the blink dog's Teleport are unmodeled), so the teleport check used a scratch blink dog with its Teleport bonus action modeled. L1 identical (100 wins, DPR 1.88). L3 100 wins both, DPR 2.12 to 1.60, rounds 1.35 to 1.36. In fights 200 to 299 at L3 the branch logs 58 teleport lines and master none: the engaged dog bites, then teleports with its bonus action (`teleports clear of 5.5 opportunity damage`).

Readings chosen where the ticket was open:

- The policy's ranged-in-close-combat Disadvantage stays a policy assumption: the engine applies no such Disadvantage. It is passed to `disposition_with` as an extra source rather than kept in the engine, and it is set exactly where it was before.
- A teleport reads engaged opponents from the current state even when the plan moves in first, since the target joins the engagement only after the move.
- Automatic critical hits on Paralyzed or Unconscious targets within 5 feet are not priced; the ticket didn't ask for it.

Follow-up tickets, filed after the audit instead of fog: [Build ranged attacks in close combat](../closed/077-build-ranged-attacks-in-close-combat.md) (077): the engine gives ranged attacks no close-combat Disadvantage while the policy assumes it unconditionally. [Build teleport subject](../tickets/078-build-teleport-subject.md) (078): the engine's `teleport` moves the plan's targets, and the policy aims a teleport plan at an opponent, so a batch teleport moves that opponent rather than the actor.

Audit fix: the ranged step-out comparison compared only damage and control, so a shot that also teleports could step out and pay the Opportunity Attacks its own teleport would avoid. It now compares `Value::worth` (damage, control and survival), tested by `a_shot_that_teleports_stays_put_rather_than_pay_the_attacks_its_teleport_avoids`. Cells were not re-measured: no SRD fixture models a teleport.
