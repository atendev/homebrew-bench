---
id: 068
title: Build attack-roll condition sources
type: task
mode: AFK
status: open
blocked_by: [067]
claimed: false
---

## Question

Build what [Align attack-roll condition sources with the 2024 conditions](../closed/067-align-attack-roll-condition-sources-with-2024-rules.md) decided: close the seven gaps between the attack disposition and PHB Appendix C, read "within 5 feet" as engaged and sight through one rule with no ranges (ADR 0017), treat a downed PC as Unconscious and Unconscious as Prone for attacks, add automatic critical hits against Paralyzed and Unconscious targets, and make damage to a Stable PC give death save failures.

## Notes

Engine (`bench-core/src/engine.rs`):

- Add `within_5_feet(a, b) -> bool`: true when `a.engagement` holds `b.id` or `b.engagement` holds `a.id`. Don't read `ranges`; a missing band defaults to engaged.
- Add `sees(state, observer, observed) -> bool`: false when `observed` has Invisible and the observer's stat block has neither `senses.blindsight` nor `senses.truesight`, or when the observer has Blinded and no `senses.blindsight`; true otherwise. Ignore the range values. Tremorsense doesn't count.
- Add one helper that says whether a combatant counts as Unconscious for attacks: it has the Unconscious condition, or its `end_state` is `Unconscious` or `Stable`. A combatant counts as Prone when it has Prone or counts as Unconscious. Don't push either into `conditions`.
- `disposition` builds its source list from these, and still passes everything to `combined_disposition`:
  - Actor Disadvantage: Blinded, Poisoned, Prone, Restrained (new).
  - Actor Grappled: Disadvantage when any Grappled instance on the actor has no `source` or a `source` other than the target's id. An actor grappled only by its target rolls straight.
  - Actor Frightened: Disadvantage when any Frightened instance whose `source` is `None`, or names a combatant still in `state.combatants` (any end state) that the actor `sees`. A source id not found in the encounter gives nothing.
  - Target Advantage: Blinded (new), Paralyzed, Petrified (new), Restrained, Stunned, counts as Unconscious.
  - Target counts as Prone: Advantage when `within_5_feet(actor, target)`, otherwise Disadvantage.
  - Target Invisible: Disadvantage only when `!sees(actor, target)`. Actor Invisible: Advantage only when `!sees(target, actor)`.
  - Timed modifiers through `modifier_dispositions`, unchanged.
- Automatic critical hits: in the `Effect::Attack` branch, a hit (natural 20 or total at or over AC, not a natural 1) is critical when the target has Paralyzed or counts as Unconscious and `within_5_feet(actor, target)`. The critical rolls the extra damage dice as now. `apply_damage` gives a dying target (`EndState::Unconscious`) two death save failures for a critical, but only that state: see the next bullet for Stable. The event message is unchanged except that a critical from this rule says so, for example `(critical: paralyzed, within 5 feet)`, so a reader sees why a roll of 12 crit.
- Damage to a Stable PC: `apply_damage` handles only `EndState::Unconscious` as down, so a Stable PC falls through to the HP branch, takes no failure and stays Stable. PHB ch. 1 "Damage and Healing", Stabilizing a Character (pp. 27-28): a Stable creature that takes damage stops being Stable and takes a death save failure, two on a critical. Treat `EndState::Stable` like `Unconscious` in that branch, set its `end_state` to `Unconscious` (dying again), and log the change, for example `mia is no longer stable`. Three failures still kill.
- `opportunity_eligibility`: replace the Invisible and Blinded checks with `sees(state, reactor, mover)`. Keep `Ineligible::Unseen` and its assumption line; the message names the reason (`c cannot see a, who is invisible` or `c is blinded and cannot see a`) as now. A reactor with Blindsight or Truesight is no longer refused against an Invisible mover, and a Blinded reactor with Blindsight is not refused.
- Make `disposition` reachable from `tactics.rs` (`pub(crate)`), because ticket 056 will call it. Don't change `tactics.rs` here.
- The attack still draws two d20s every time. The dice stream moves only where a new critical rolls extra damage dice.

Docs:

- ADR 0015: amend the eligibility bullet's sentence "The engine also assumes Invisible hides the mover from every reactor and that a Blinded reactor sees nothing" to point at ADR 0017's sight rule.
- ADR 0005, the `if_target` paragraph: no change needed, but check that its advice to set the grappler as `source` still reads right.
- `.agents/skills/homebrew-bench/references/live-mode.md`, the Opportunity attacks bullet (line 58): the reasons a reactor can't react change to "can't see the mover (ADR 0017)". Add one short paragraph near the override examples. Warn first that the engine reads engagement for these rules and doesn't check it before a melee attack: live combatants start near (line 10), so an agent who resolves a melee attack without `bench move` to engage gives the attacker Disadvantage against a Prone target and no automatic critical hit against a Paralyzed or downed one. Then: name the grappler or frightener in a condition's `source` when setting one by override, because the engine reads it; a downed PC is treated as Unconscious and Prone for attacks; Prone and automatic critical hits read "within 5 feet" as engaged; and See Invisibility or a "sees invisible creatures" trait is agent judgment (remove the Invisible condition by override for that attack).
- `CHANGELOG.md` under Changed: attack rolls now follow the 2024 conditions for Blinded and Petrified targets, Restrained and Grappled attackers, Prone targets attacked from beyond engagement, Frightened attackers who can't see the source, Invisible creatures facing Blindsight or Truesight, and downed PCs; damage to a Stable PC now gives a death save failure and makes it dying again; hits on Paralyzed and Unconscious targets from an engaged attacker are critical; Opportunity Attacks use the same sight rule; batch numbers change; and the engine version moves.

Tests (`tdd`), each a small state built in `engine.rs` tests:

- Blinded target gives Advantage; Petrified target gives Advantage; Restrained attacker gives Disadvantage.
- Prone target: engaged attacker has Advantage; attacker in the near band has Disadvantage; engagement recorded only on the target's side still counts.
- Grappled attacker: straight against its grappler (`source` = target), Disadvantage against another target, Disadvantage against everyone when `source` is `None`.
- Frightened attacker: Disadvantage while the source is present and seen; straight when the source is Invisible and the attacker lacks Blindsight and Truesight; straight when the source id is not in the encounter; Disadvantage when the source is Dead; Disadvantage with no source.
- Invisible: attacker with Advantage against a plain target, straight against a target with Truesight, straight against a target with Blindsight; Invisible target gives Disadvantage to a plain attacker and nothing to one with Blindsight. Tremorsense alone changes nothing.
- Downed PC (`end_state` Unconscious, no conditions): engaged attacker has Advantage and its hit is critical with two death save failures; an attacker in the far band rolls straight and its hit is not critical.
- Stable PC: any damage gives one death save failure and makes it dying (`EndState::Unconscious`) again; an engaged critical gives two; a third failure kills.
- Paralyzed target: engaged hit on a natural 2 that beats AC is critical and rolls double dice; the same roll from the near band is not.
- Opportunity Attacks: a reactor with Truesight may attack an Invisible mover; a Blinded reactor with Blindsight may react; a plain Blinded reactor is still refused with the assumption line.
- Existing tests (`disposition_blinded_gives_disadvantage`, the `combined_disposition` tests, the Opportunity Attack sight tests) still pass or are updated only where this ticket changes the rule.

Versions and baseline:

- Bump the workspace version, and with it `ENGINE_VERSION`, by one patch from what `master` holds when this lands. Batch numbers change. Don't regenerate `data/baseline/curve.json`. The engine stale reason is already accepted by `stale_beyond_the_engine` in `bench-core/tests/calibration.rs` and by the CLI test `batch_compares_against_the_embedded_curve_outside_the_repo`.
- `a_fixture_regenerates_to_its_committed_row` in `bench-core/tests/calibration.rs` regenerates goblin-warrior and compares it with the committed curve row. If the row differs and the test is not ignored yet, add `#[ignore = "curve stale until ticket 056 regenerates it"]`. Don't edit the committed row. Ticket 056 removes the ignore.
- Ticket 056 waits on this ticket so its one regeneration covers the change.
- Ticket [063](../closed/063-build-batch-reactions-policy.md) exposes `opportunity_eligibility` to the policy (through `engine::opportunity_eligible`), and its step-out penalty and tests read that rule, which this ticket changes. Whichever lands second rebases. 063's penalty test that a Blinded reactor prices 0 still holds only for a reactor without Blindsight; if 063 lands first, check that test's reactor has none, and if this lands first, 063 builds that test on the new sight rule.
- Tickets 059, 060, 062, 065, 066 and 069 also edit `engine.rs` and `CHANGELOG.md`, and several bump the version. None has to land first; whichever lands second rebases. Ticket 060 adds lighting dispositions to the same source list; keep both. Ticket 069 reuses the "counts as Unconscious" helper.

Consult `tdd`, `codebase-design`, `domain-modeling` and `code-review`.

## Resolution
