---
id: 078
title: Build teleport subject
type: task
mode: AFK
status: closed
blocked_by: [077]
claimed: false
---

## Question

Found by [Build v3 teleport scoring and engine-sourced policy dispositions](../closed/073-build-v3-teleport-scoring-and-engine-sourced-dispositions.md) and confirmed by its audit (severity Medium). A teleport doesn't say who moves. `effect::Teleport` (`bench-core/src/effect.rs`, about line 405) has only `range` and `destination`, and the engine's `Effect::Teleport` arm (`bench-core/src/engine.rs`, about line 676) clears engagement for the command's targets. Batch aims every plan at an opponent, so a batch teleport moves that opponent, and in live mode a teleport used with no target does nothing. 073 scores a teleport as the Opportunity Attacks the actor avoids from everyone engaged with it, so the score and the result disagree once two foes are engaged. Failure case: a homebrew blinker engaged by a fighter and a paladin teleports. The policy credits both Opportunity Attacks as avoided, but the engine moves only the fighter, the blinker stays engaged with the paladin, and the fighter loses its engagements with every other monster too. Make a teleport move its subject, the actor by default. `POLICY_VERSION` stays 3. Bump the workspace version one patch. Don't regenerate `data/baseline/curve.json`.

No SRD fixture or standard-party file models a teleport today: the blink dog's and marilith's Teleport and the mage's, archmage's and cloud giant's Misty Step import as unmodeled, the balor's and nalfeshnee's teleports exist only as text, and the party casts none. Only `data/spells/effects.json` (`misty-step`, with a `teleport` effect) and homebrew stat blocks carry one. So SRD batch numbers shouldn't move, and the change is in what homebrew teleports do.

## Notes

Consult `tdd`, `codebase-design`, `domain-modeling` and `code-review`. Read ADR 0005 (the closed effect vocabulary), ADR 0015 ("forced movement, teleports and MoveThenAttack never provoke"), ADR 0012's "Teleports and roll sources" paragraph, and how [Build modifier bearers and Speed modifiers](../closed/065-build-modifier-bearers-and-speed-modifiers.md) added `bearer` to `timed_modifier` with a default that kept old data loading.

Vocabulary (`bench-core/src/effect.rs`):

- Add `subject` to `Teleport`: `"self"` (the creature using the ability) or `"target"` (each target the effect resolves against). Reuse the shape of `ModifierBearer` (serde `"self"` / `"target"`), or reuse that enum outright if its doc comment can say both without strain. The default is `self`, unlike a modifier's `target` default: "teleports up to 40 feet" in a stat block means the creature itself, and Misty Step is self-only.
- An explicit `"target"` is allowed only where ADR 0005's vocabulary already lets an effect land on a target: at the top level of an ability used with targets, and under an attack's `on_hit` or a save's `on_failure` ("the target is teleported 30 feet"). Decide whether a top-level `target` teleport with no target in the command is an error (as a top-level modifier without a bearer needs a target), and match whatever `bench schema validate` does for modifiers.
- Serde default `self` with `skip_serializing_if` for the default, so existing JSON (`effects.json`'s Misty Step, homebrew blocks, saved encounter states and command histories) loads unchanged and re-serializes byte-identical. Add an old-state fixture test only if a stored state can hold a `Teleport`; a stat-block snapshot inside a state can, so check `StatBlockSnapshot` round-trips.

Engine (`bench-core/src/engine.rs`):

- `Effect::Teleport` resolves on its subject: the actor for `self`, each target for `target`. It clears all of the subject's engagements on both sides (the subject's own list, and the subject's id from everyone else's list), as today's arm does for a target. It never provokes (ADR 0015). `ranges` handling: say what band the subject ends at relative to those it left (today nothing changes `ranges`; `near` matches a step-out). Log a `teleport` event, such as `<id> teleports up to 30 feet`, since today the arm logs nothing.
- A `self` teleport inside a Multiattack, a cast or a `use_action` moves the actor, whatever targets the command named.
- Live mode: `bench action <actor> <teleport-ability>` with no target now teleports the actor.

Policy (`bench-core/src/tactics.rs`):

- `teleport_value` (073) is right for a `self` teleport. A `target` teleport moves an opponent away: score it 0 here, as 045 left pulls, and add a fog line in `map.md` for its value (it is close to a push to far).
- A plan for an ability whose only effects are `self` teleports targets nobody, like a `self` modifier plan, and logs no target in its reason. An ability that also attacks keeps the attack's target. Check `batch::take_turn` and `execute` accept an empty target list for such a plan.
- Re-check 073's teleport tests: a teleport with two engaged foes now frees the actor from both, which is what the score already says. Add a test that the policy's credited avoided attacks equal what the engine frees.

Import and scripts:

- `scripts/scaffold-spells.py` writes `{"teleport": {"range": N, "destination": ...}}` from spell text. Keep that (the default is `self`), or write `subject` only for spells whose text teleports a target. `scripts/check-spell-effects.py` prints `teleport up to N feet`; add the subject when it isn't `self`. Re-run both and confirm `effects.json` and the check report are unchanged unless a spell's subject really is the target.
- `scripts/import-srd-monsters.py` imports no teleport today. Don't make the SRD Teleport actions modeled here; that is an import decision for a later ticket. It must still exit 0 with 341 of 341 graduated.
- The JSON schema (`bench schema`) gains the optional field.

Docs:

- ADR 0005: amend the vocabulary list so a teleport names its subject, default self, and say where `target` is allowed.
- ADR 0015: no change to "teleports never provoke"; check it still reads right.
- ADR 0012, "Teleports and roll sources": a teleport plan moves the actor, and a `target` teleport scores 0.
- `.agents/skills/homebrew-bench/references/batch-mode.md` (where `bearer` is explained) and the live-mode reference: one sentence each on `subject`. If the shell guard holds an edit to these files, don't work around it; record it in the Resolution.
- `CHANGELOG.md`: under Added, the `subject` field; under Changed, a teleport now moves the creature using it by default and clears all its engagements, batch and live.

Tests (`tdd`):

- Engine: a `self` teleport used with an opponent as target frees the actor from two engaged foes and leaves the opponent's other engagements alone; a `target` teleport under `on_hit` frees the hit target; a `self` teleport in a Multiattack frees the actor; a teleport with no target in live mode moves the actor; old JSON without `subject` loads as `self` and serializes without it; the event is logged.
- Policy: the blinker engaged by two foes gets a teleport plan whose credited value is both Opportunity Attacks, and after `execute` it is engaged with nobody; a `target` teleport scores 0; a teleport-only plan has no targets.
- Batch: a seed-fixed run with a scratch homebrew teleporter (073 used a blink dog with Teleport modeled) where the log shows the actor teleporting.

Versions and baseline:

- `POLICY_VERSION` stays 3. Bump the workspace version, and with it `ENGINE_VERSION`, one patch from what master holds when this lands. The stat-block vocabulary gains an optional field, so confirm older encounter states still load and replay.
- Don't regenerate the curve. If `a_fixture_regenerates_to_its_committed_row` differs and is not ignored yet, add `#[ignore = "curve stale until ticket 074 regenerates it"]`.
- Measure before closing: master against the branch, seed 7, 100 fights, for adult red dragon at 11 and 17 (expect identical) and the scratch teleporter at two levels.

## Resolution

`Teleport.subject` is `self` (the default) or `target`, left out of the JSON when `self`, so older stat blocks, snapshots and histories load and re-serialize unchanged (a snapshot round-trip test covers it). The engine resolves a teleport on its subject, clears its engagements on both sides, leaves range bands alone, and logs `<id> teleports up to N feet`. A top-level `target` teleport with no target is refused. The policy plans an ability whose effects are all `self` teleports, reached through a cast too, with no targets, scored as the Opportunity Attacks avoided; a `target` teleport scores 0 and its value is a new fog line. A test checks the credited value equals what the engine frees. `POLICY_VERSION` stays 3, the workspace is 0.1.15, the curve is not regenerated, and ADR 0005 and 0012, the batch and live references and `CHANGELOG.md` say so.

Seed 7, 100 fights, master against the branch: adult red dragon at levels 11 and 17 and a scratch blink dog with a modeled Teleport at levels 2 and 5 are identical. With a weakened bite at level 5 the scratch blinker teleports on the branch and the log shows it.

Audit: haiku did not approve in two rounds. Round 1 problems (casts bypassing the new plan, a misplaced doc comment, no snapshot test, no fog line) were fixed. Round 2 left: `target` is checked only at runtime, not restricted at load to the positions ADR 0005 names, so under a Multiattack with a target it moves that target; no engine test for a self teleport in a Cast or `use_action`; the refusal message doesn't name the ability; a self teleport under a per-target `on_hit` logs once per target. The user chose to commit and merge as it stands. Requirements report: `.plan/reports/078-requirements.md`. No implementation report was written. No shell guard held a skill edit; one held test edit was approved and run.

