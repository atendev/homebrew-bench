---
id: 077
title: Build ranged attacks in close combat
type: task
mode: AFK
status: closed
blocked_by: [076]
claimed: false
---

## Question

Found by [Build v3 teleport scoring and engine-sourced policy dispositions](../closed/073-build-v3-teleport-scoring-and-engine-sourced-dispositions.md) and confirmed by its audit (severity High). The 2024 PHB, ch. 1 "Ranged Attacks in Close Combat" (p. 25): "When you make a ranged attack roll, you have Disadvantage on the roll if you are within 5 feet of an enemy who can see you and who doesn't have the Incapacitated condition." The engine has no such source: `engine::disposition_with` (`bench-core/src/engine.rs`, about line 2337) never reads the attack. The batch policy adds its own version as an extra source (`Footing::ranged_in_melee` in `bench-core/src/tactics.rs`, set in `plan_for` around line 592 and read in `effect_value`'s attack arm around line 970), and that version ignores sight and Incapacitated. So engine and policy disagree for every engaged ranged attacker. The standard party's strikers and defenders carry ranged attacks, so standard-party cells move. Add the source in the engine and make the policy read it. `POLICY_VERSION` stays 3. Batch numbers change, so bump the workspace version one patch. Don't regenerate `data/baseline/curve.json`.

## Notes

Consult `tdd`, `codebase-design`, `domain-modeling` and `code-review`. Read ADR 0017 (position-worded rules read through engagement and senses), the Resolution of [Build attack-roll condition sources](../closed/068-build-attack-roll-condition-sources.md) for the same shape of change, and ADR 0012's "Teleports and roll sources" paragraph.

Engine (`bench-core/src/engine.rs`):

- Ranged or melee. `effect::Attack` records `reach: Option<u32>` and `range: Option<..>`. `AttackFilter::Ranged` (hit triggers) reads `range.is_some()`, but a thrown weapon (a dagger or handaxe) has both, and a creature within reach uses it as a melee attack. For this rule an attack roll is ranged when `range.is_some() && reach.is_none()`, which is what the policy reads today. Add one small helper, such as `Attack::is_ranged_only()` in `effect.rs`, and use it in both places. Leave `AttackFilter` unchanged and say so in a doc comment, so a later reader sees the two readings are deliberate.
- The source. `disposition` and `disposition_with` take the attack (or a `ranged: bool` derived from it); every caller passes it. Today the only engine caller is the `Effect::Attack` branch of `resolve_effect` (about line 499), which Opportunity Attacks also reach; the rest are tests and the policy. A ranged attack roll has Disadvantage when any combatant within 5 feet of the attacker is an enemy that sees it and can act:
  - "Within 5 feet" is engagement (ADR 0017), the same reading `within_5_feet` gives, never `ranges`.
  - Enemy: a combatant on another side (`side_id` differs) whose `end_state` is `Active`. Dead creatures are not enemies here, and downed PCs count as Unconscious and so are Incapacitated.
  - "Can see you": `sees(state, enemy, attacker)`, ADR 0017's one sight rule. An Invisible attacker is unseen by an enemy without Blindsight or Truesight; a Blinded enemy without Blindsight sees nothing.
  - "Doesn't have the Incapacitated condition": Incapacitated itself or a condition that includes it, which are Paralyzed, Petrified, Stunned and Unconscious, plus `counts_as_unconscious`. Keep that list in one place, shared with the policy's `can_act` (076 also asks for one shared list; reuse whatever 076 leaves rather than adding a third copy).
- `disposition_with`'s `within: bool` and `extra` parameters were for the policy. Once the engine owns the rule, replace them with what the policy really needs: who is within 5 feet of the attacker after the plan moves. One way is a `Reach` (or similar) argument that says, for the attacker, which combatant ids it is engaged with; `disposition` passes the attacker's current engagement, and the policy passes engagement as the plan leaves it (`Moves::Stays` as held, `Moves::In` adds the target, `Moves::Out` none). The target's Prone reading uses the same list. Drop `extra` if nothing else uses it.
- The disposition note on the attack event names the source when the roll goes that way, such as `(Disadvantage: ranged attack in close combat)`. Today attack notes name only lighting sources (069's reading); add this one beside them, since a reader can't otherwise see why a bow shot rolled twice.
- The two d20s are still drawn every time, so the dice stream doesn't move except through changed hits.

Policy (`bench-core/src/tactics.rs`):

- Remove `Footing::ranged_in_melee` and its computation in `plan_for`. The policy reads the source only through the engine's disposition, with engagement as the plan leaves it.
- The ranged step-out comparison stays as it is (stepping out of every engagement drops the source), but it must now price the Disadvantage the engine will really impose: none when every engaged foe can't see the shooter or is Incapacitated. Keep 073's `Value::worth` comparison.
- Re-run 063's and 073's step-out and teleport tests. Where a test relied on the old unconditional Disadvantage, keep its assertion and change the state, not the expected value.

Docs:

- ADR 0017: add the rule to its list of position-worded rules read through engagement and `sees`.
- ADR 0012, the "Teleports and roll sources" paragraph: the close-combat Disadvantage is now the engine's, not a policy source.
- `.agents/skills/homebrew-bench/references/live-mode.md`, the paragraph on engagement and condition rules (line 66): a ranged attack by an engaged creature has Disadvantage unless every engaged enemy can't see it or is Incapacitated; `bench move` out of engagement first, or Disengage, to shoot straight. Ticket 069's builder found that the shell guard can hold edits to this file. If it does, don't work around it: raise it in the Resolution as ticket 072 did.
- `CHANGELOG.md` under Changed: ranged attack rolls made within 5 feet of an enemy that can see the attacker and isn't Incapacitated have Disadvantage, in live and batch mode, and batch numbers change.
- `CONTEXT.md` **Engaged** already says "within 5 feet" reads as engaged. Check it still matches.

Tests (`tdd`), small states in `engine.rs` and `tactics.rs` tests:

- Engine: an engaged archer's bow has Disadvantage; its dagger with reach and range doesn't; an archer only in the near band rolls straight; an engaged enemy that is Incapacitated, Paralyzed, Stunned, Unconscious, downed or Dead gives nothing; an Invisible archer against an enemy without Blindsight or Truesight rolls straight, and against one with Truesight has Disadvantage; a Blinded enemy without Blindsight gives nothing; an engaged ally gives nothing; two engaged enemies give one Disadvantage, and Advantage from another source (a Restrained target) makes it straight; engagement recorded only on the enemy's side counts; the attack event note names the source.
- Policy: a ranged plan by an engaged archer scores Disadvantage; the same plan with the engaged brute Stunned scores straight and doesn't step out; a step-out still happens when it pays; a melee plan that moves in beside a second foe and also carries a ranged attack reads the engine's reading.
- Batch: a seed-fixed run where an engaged standard striker shoots and its attack note shows the source.

Versions and baseline:

- `POLICY_VERSION` stays 3. Bump the workspace version, and with it `ENGINE_VERSION`, one patch from what master holds when this lands. No live-state field changes, so no old-state fixture test.
- Don't regenerate the curve. If `a_fixture_regenerates_to_its_committed_row` differs and is not ignored yet, add `#[ignore = "curve stale until ticket 074 regenerates it"]`.
- Measure before closing, as 073 did: release builds of master and the branch from `git archive` in a temp dir with `HOME=$(mktemp -d)`, `bench batch <id> --levels <L> --fights 100 --seed 7` for goblin warrior, knight, ogre and adult red dragon at levels 5, 11 and 17. Report wins, PC deaths, monster DPR mean and rounds mean.

## Resolution

Built as asked. Engine 0.1.14, `POLICY_VERSION` stays 3, curve not regenerated.

- Engine. `Attack::is_ranged_only` (`effect.rs`) is the rule's reading: a range and no reach. `AttackFilter` is unchanged, and its doc comment says the two readings differ on purpose. `disposition(state, actor, target, attack)` and `disposition_with(state, actor, target, attack, within)` replace the old `within: bool` and `extra` parameters with `engine::Within`: `Held` (engagement on either side), `HeldAnd(id)` (held plus the target the plan moves in to) and `Nobody` (stepped out). The target's Prone and the new `in_close_combat` source both read it. An enemy counts when it is on another side, within, `can_act` (Active and no `STOPS_ACTING` condition, which covers downed and dead) and `sees` the attacker. The attack event adds `(Disadvantage: ranged attack in close combat)` beside lighting sources when the roll goes that way. Both d20s are still drawn every time.
- Policy. `Footing::ranged_in_melee` and its computation are gone; `effect_value` maps `Moves` to `Within` and calls the engine. The policy's copy of `can_act` is gone; it uses the engine's, now `pub(crate)`, so one function reads `STOPS_ACTING`. The step-out comparison is unchanged and now prices the engine's real Disadvantage: an archer engaged only with an Incapacitated brute shoots straight and stays. 063's and 073's step-out and teleport tests pass with no state changes.
- Tests. Engine: bow vs. thrown dagger vs. sword; near band; engagement recorded only on the enemy; Incapacitated, Paralyzed, Stunned, Unconscious, Petrified, downed, stable and dead enemies; Invisible archer with and without Truesight; Blinded enemy; engaged ally; two enemies and a cancelling Restrained target; `disposition_with` over each `Within`; the event note. Policy: engaged archer scores Disadvantage and stays; Incapacitated brute scores straight and stays; a shot read under each footing. Batch: an engaged level 1 standard striker shooting the ogre (seed-fixed) logs the source.
- Deviation. The ticket's policy case "a melee plan that moves in and also carries a ranged attack" can't arise through `plan_for`: `needs_engaged` moves in only when every attack is melee. The test checks `effect_value` directly under `Moves::In` instead. The ticket's "Stunned brute" case is tested with Incapacitated, because Stunned also gives the shot Advantage against the brute.
- Docs. ADR 0017 lists the rule; ADR 0012's "Teleports and roll sources" says the source is the engine's; live-mode.md's engagement paragraph gains the rule (the shell guard did not hold the edit); CHANGELOG under Changed. CONTEXT.md **Engaged** still matches.
- Checks. `cargo test --workspace` passes; `a_fixture_regenerates_to_its_committed_row` still matches, so no `#[ignore]`. Clippy shows only the two known engine.rs errors. `python3 -I scripts/import-srd-monsters.py` exits 0 with 341 of 341.

Seed 7, 100 fights, master (0.1.13) to branch, as wins / PC deaths / monster DPR mean / rounds mean. Goblin warrior and centaur trooper refuse to batch without assumptions (unmodeled attacks) on both builds, so they are left out.

| Cell | Master | Branch |
|---|---|---|
| knight L5, L11, L17 | 100/0/7.56/2.09, 100/0/4.18/1.19, 100/0/3.86/1.07 | identical |
| ogre L5, L17 | 100/0/3.97/1.95, 100/0/1.20/1.05 | identical |
| ogre L11 | 100/0/1.61/1.18 | 100/0/1.61/1.19 |
| adult red dragon L5, L11, L17 | 0/100/123.14/2.29, 43/57/96.18/4.70, 99/1/114.28/2.94 | identical |
| scout L5, L11, L17 | 100/0/2.50/1.04, 100/0/1.67/1.00, 100/0/0.77/1.00 | identical |
| ogre L1 | 91/13/4.52/4.35 | 91/13/4.47/4.40 |
| ogre L3 | 100/0/4.05/2.56 | 100/0/4.33/2.78 |
| knight L1 | 21/84/13.69/5.15 | 23/82/13.40/5.12 |
| knight L3 | 85/16/11.00/4.83 | 85/16/11.13/4.81 |
| gnoll warrior L1 | 100/1/1.87/1.96 | 100/1/1.84/1.97 |
| bandit captain L1 | 91/12/5.42/3.91 | 91/12/5.43/3.90 |
| bandit captain L3 | 100/0/4.86/2.63 | 100/0/4.68/2.66 |

Scout L1 and L3 and gnoll warrior L3 are identical. The shift is small. The policy already scored engaged shots at Disadvantage, so engaged shooters mostly stepped out or used a melee attack, and engaged shots were rare. Cells move where an engaged shot still happens (the engine now rolls it at Disadvantage, where it rolled straight before) or where the engaged foe can't act or see the shooter (the policy now scores it straight).

No new tickets or fog. With 077 closed, [Build teleport subject](../tickets/078-build-teleport-subject.md) is takeable.
