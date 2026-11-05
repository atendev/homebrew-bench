# Ticket 078 requirements report

Ticket: `.plan/tickets/078-build-teleport-subject.md`. Finding source: `.plan/closed/073-build-v3-teleport-scoring-and-engine-sourced-dispositions.md` (audit severity Medium). Modifier bearer precedent: `.plan/closed/065-build-modifier-bearers-and-speed-modifiers.md`.

## Goal

A teleport moves its subject, the creature using the ability by default, not whichever targets the command named. Today `Effect::Teleport` carries only `range` and `destination`, so a batch plan aimed at an opponent moves that opponent, and a live teleport with no target does nothing. Ticket 073 scores a teleport as the Opportunity Attacks the actor avoids, so score and result disagree once two foes are engaged. Failure case: a homebrew blinker engaged by a fighter and a paladin teleports. The policy credits both attacks as avoided, but the engine moves only the fighter, so the blinker stays engaged with the paladin and the fighter loses its engagements with every other monster.

## Acceptance criteria

- [ ] `Teleport` gains `subject`: `self` (default) or `target`. Serde shape follows `ModifierBearer` (`effect.rs:307`), or reuses that enum if its doc comment fits both uses. The field is skipped when it holds the default.
- [ ] Existing JSON loads unchanged and re-serializes byte-identical: `misty-step` in `data/spells/effects.json`, homebrew stat blocks, saved encounter states and command histories. Check that `StatBlockSnapshot` round-trips. Add an old-state fixture test only if a stored state can hold a `Teleport`.
- [ ] Explicit `target` is allowed only where ADR 0005 already lands effects on a target: the top level of an ability used with targets, or under an attack's `on_hit` or a save's `on_failure`.
- [ ] Decide whether a top-level `target` teleport with no target in the command is an error. Match what `bench schema validate` does for a top-level modifier with no bearer.
- [ ] Engine: `Effect::Teleport` resolves on its subject (the actor for `self`, each target for `target`). It clears all of the subject's engagements on both sides. It never provokes (ADR 0015). It logs a `teleport` event, such as `<id> teleports up to 30 feet`; today the arm logs nothing.
- [ ] Engine: state the band the subject ends at relative to creatures it left (`near` matches a step-out). Today nothing changes `ranges`, so this is a decision to record.
- [ ] A `self` teleport inside a Multiattack, a cast or `use_action` moves the actor whatever targets the command named.
- [ ] Live mode: `bench action <actor> <teleport-ability>` with no target teleports the actor.
- [ ] Policy: `teleport_value` is unchanged for `self`. A `target` teleport scores 0, as 045 left pulls. A plan whose only effects are `self` teleports has no targets and logs none in its reason. An ability that also attacks keeps the attack's target. `batch::take_turn` and `execute` accept an empty target list.
- [ ] Test: the policy's credited avoided attacks equal what the engine frees.
- [ ] Scripts: `scaffold-spells.py` writes `subject` only for spells whose text teleports a target. `check-spell-effects.py` prints the subject when it is not `self`. Re-run both. `effects.json` and the check report change only if a spell's subject is the target.
- [ ] `scripts/import-srd-monsters.py` still exits 0 with 341 of 341 graduated. No SRD Teleport action is modeled here.
- [ ] `bench schema` shows the optional field.
- [ ] Docs: ADR 0005 names `subject` (default `self`, and where `target` is allowed). ADR 0015 is re-read, expected unchanged. ADR 0012 "Teleports and roll sources" says a teleport plan moves the actor and a `target` teleport scores 0. `batch-mode.md` and `live-mode.md` each gain one sentence on `subject`. `CHANGELOG.md` Unreleased: Added the `subject` field; Changed: a teleport moves the creature using it by default and clears all its engagements, batch and live.
- [ ] Versions: `POLICY_VERSION` stays 3. The workspace version goes up one patch from master's value when this lands (`Cargo.toml` reads 0.1.14 on this branch). `ENGINE_VERSION` follows through `CARGO_PKG_VERSION`.
- [ ] `data/baseline/curve.json` is not regenerated. If `a_fixture_regenerates_to_its_committed_row` differs and is not yet ignored, add `#[ignore = "curve stale until ticket 074 regenerates it"]`.
- [ ] Before closing, measure master against the branch with seed 7 and 100 fights. Adult red dragon at CR 11 and 17 should be identical. Report the scratch teleporter at two levels.
- [ ] Tests (`tdd`), engine: a `self` teleport aimed at an opponent frees the actor from two engaged foes and leaves the opponent's other engagements alone; a `target` teleport under `on_hit` frees the hit target; a `self` teleport in a Multiattack frees the actor; a no-target live teleport moves the actor; old JSON without `subject` loads as `self` and re-serializes without it; the event is logged.
- [ ] Tests, policy: the blinker with two engaged foes gets a plan credited with both attacks and is engaged with nobody after `execute`; a `target` teleport scores 0; a teleport-only plan has no targets.
- [ ] Tests, batch: a seed-fixed run with a scratch homebrew teleporter whose log shows the actor teleporting.
- [ ] If the shell guard holds an edit to a reference file, do not work around it. Record it in the ticket's Resolution.

## Constraints

- ADR 0005 closed vocabulary: no new effect kind. `subject` is an optional field on an existing one.
- Default `self` is deliberate and differs from `ModifierBearer`'s default `target`. "Teleports up to 40 feet" means the creature itself, and Misty Step is self-only.
- Teleports never provoke (ADR 0015). No change to that rule.
- `POLICY_VERSION` stays 3. SRD batch numbers should not move: no SRD fixture models a teleport today.
- Older encounter states must still load and replay under the bumped version. `store.rs` (`compatible`, ~line 191) decides which recorded engine versions may finish a state, so check it against the new version.
- SRD teleport actions stay unmodeled; importing them is a later ticket.
- Do not regenerate `data/baseline/curve.json`.

## Code locations

Checked with grep on this branch. Three refs in the ticket were off and are corrected below.

- `bench-core/src/effect.rs:420`: `pub struct Teleport { range, destination }`, doc comment at 418. The ticket said about line 405. Variant `Effect::Teleport(Teleport)` at 513.
- `bench-core/src/effect.rs:307`: `pub enum ModifierBearer` (default `Target`, `Actor` serialized as `self`). `is_target` at 320. Pattern to copy: `timed_modifier.bearer` at 293-294 with `skip_serializing_if`.
- `bench-core/src/engine.rs:680`: `Effect::Teleport(_)` arm. It clears engagement for each target id on both sides and logs nothing. The ticket said about line 676.
- `bench-core/src/engine.rs:2735`: `event()` helper. It sets `player_dice: false` and `roll: None`.
- `bench-core/src/engine.rs:1675-1677` and `2274-2287`: where `ranges` is written, for the band decision.
- `bench-core/src/encounter.rs:98`: `StatBlockSnapshot`. `encounter.rs:138`: `ranges: HashMap<String, RangeBand>`.
- `bench-core/src/tactics.rs:21`: `POLICY_VERSION = 3`.
- `bench-core/src/tactics.rs:1154`: `Effect::Teleport` value arm. `tactics.rs:1181`: `teleport_value` (073), returns 0 when the footing moves out.
- `bench-core/src/tactics.rs:2884` and `2893-2920`: 073's `teleport_value` helper and tests.
- `bench-core/src/batch.rs:690` `take_turn` and `:745` `execute`: both need empty-target handling.
- `bench-core/src/live.rs:401`: live `execute`.
- `bench-core/src/store.rs:191`: engine version compatibility check.
- `bench-core/src/lib.rs:31`: `ENGINE_VERSION = env!("CARGO_PKG_VERSION")`. `Cargo.toml:6`: `version = "0.1.14"`.
- `scripts/scaffold-spells.py`, `scripts/check-spell-effects.py`, `scripts/import-srd-monsters.py`.
- `data/spells/effects.json` (`misty-step`). `data/baseline/curve.json` (do not regenerate).
- `docs/adr/0005-closed-effect-vocabulary-with-agent-clauses.md:3` (vocabulary list). `docs/adr/0012-deterministic-batch-tactics-policy.md:15` ("Teleports and roll sources"). `docs/adr/0015-opportunity-attacks-declared-on-the-move.md:29`.
- `.agents/skills/homebrew-bench/references/batch-mode.md` and `live-mode.md`.
- `CHANGELOG.md`: Unreleased at line 3, Added at 5, Changed at 27.
