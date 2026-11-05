---
id: 073
title: Build v3 teleport scoring and engine-sourced policy dispositions
type: task
mode: AFK
status: open
blocked_by: [056]
claimed: false
---

## Question

Split from [Build v3 scoring for buffs and forced movement](056-build-v3-scoring-for-buffs-forced-movement-and-teleports.md) for size. In `bench-core/src/tactics.rs`, score teleports by the Opportunity Attacks they avoid, and make the policy read attack and save dispositions from the engine instead of its own partial lists. Amend ADR 0012 where these change it. `POLICY_VERSION` stays 3. Batch numbers change, so bump the workspace version one patch. Don't regenerate `data/baseline/curve.json`. If `a_fixture_regenerates_to_its_committed_row` differs, add `#[ignore = "curve stale until ticket 074 regenerates it"]`.

## Notes

Consult `tdd`, `codebase-design`, `domain-modeling`.

- Teleports score the Opportunity Attacks they avoid by calling 063's penalty function (`Ctx::step_out_penalty`): eligible engaged reactors the policy would attack with, each at its `opportunity_value`. This replaces ticket 045's "sum of `best_single_attack` over engaged foes that still have their reaction", because 046 decided the mover prices only the attacks the policy really takes. The penalty and the reserve return 0 at or past `MAX_THREAT_NESTING` (063 builds that guard), because the teleport score calls the penalty, which reaches `threat()` and `ability_value` and can come back to a teleport. Keep the guard, and add a test: a stat block whose actions include a teleport and a Multiattack that uses it, engaged with a reactor that has Parry, gets a plan without overflowing the stack, as the Dryad test does for control.
- Fix `disposition` in `tactics.rs`: it sums and clamps, so an attacker with two Advantage sources and one Disadvantage source scores as Advantage. Use the two-flag rule from ticket 061 (both or neither is straight), with a test that two of one and one of the other score straight.
- The policy's `disposition` must also read the sources [Align attack-roll condition sources](../closed/067-align-attack-roll-condition-sources-with-2024-rules.md) added (ADR 0017): Prone by engagement, Grappled and Frightened by `source`, Invisible through the sight rule, downed PCs as Unconscious. Call the engine's `disposition` (ticket [068](../closed/068-build-attack-roll-condition-sources.md) makes it `pub(crate)`) instead of keeping a second list, reading engagement as the plan leaves it when the plan moves, and test that a ranged plan against a Prone target scores Disadvantage.
- The policy's saving-throw value (`p_success` in `effect_value`) reads no save sources: it misses the automatic Strength and Dexterity failures, Restrained's Dexterity Disadvantage, lighting and timed save modifiers (Haste). Ticket [069](../closed/069-build-saving-throw-condition-sources.md) makes the engine's `automatic_save_failure`, `save_disposition` and `save_modifier` `pub(crate)`; call them instead of a second list, and test that a Dexterity save against a Paralyzed target scores as a certain failure.

## Resolution
