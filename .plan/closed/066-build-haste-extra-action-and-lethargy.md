---
id: 066
title: Build Haste extra action and lethargy
type: task
mode: AFK
status: closed
blocked_by: [065]
claimed: false
---

## Question

Build decision 7 of [Haste extra action and ally-targeted buffs](../closed/055-haste-extra-action-and-ally-targeted-buffs.md): a modifier can grant its bearer an extra action each turn, limited to a list of uses, and a modifier can apply effects to its bearer when it ends. Haste uses both: one extra action for one attack or Disengage, and lethargy when the spell ends. The batch policy's use of the extra action is built by ticket 056 under v3, not here.

## Notes

Consult `tdd`, `codebase-design`, `domain-modeling` and `code-review`. Read ADR 0004, ADR 0005 and ADR 0015 first. Builds on ticket 065's `bearer`, the stacking accessor `EncounterState::modifiers_on` and the stored `source_ability`.

Rules (PHB p. 283): the target "gains an additional action on each of its turns. That action can be used to take only the Attack (one attack only), Dash, Disengage, Hide, or Utilize action." When the spell ends, "the target is Incapacitated and has a Speed of 0 until the end of its next turn". Dash has no engine command, and Hide and Utilize are out of combat math, so those three stay an agent clause.

Model (`bench-core/src/effect.rs`, `bench-core/src/encounter.rs`):
- `ModifierTarget::ExtraAction { only: Vec<ExtraActionUse> }`, with `ExtraActionUse` = `one_attack` or `disengage`. An empty `only` means any action, for homebrew "gains an additional action" riders. The change is `none`. Stacking (ticket 065's rule) applies, so two Hastes on one bearer still give one extra action.
- `TimedModifier.on_end: Vec<Effect>`, serde default empty, skipped when empty. When that modifier instance ends (its duration runs out, its source's concentration ends, or it ends on damage), the engine resolves each effect with the modifier's source as actor and the bearer as the one target. It does not fire when the encounter ends. `ActiveModifier` stores `on_end` the same way, skipped when empty, so saved states keep their hashes (see ticket 065).
- `Duration::UntilEndOfNextTurn`: lasts until the end of the bearer's next turn. If it is applied during the bearer's own turn, that turn does not count. The build picks the storage (for example a flag set when the bearer's next turn starts); it must serialize nothing extra on old states.
- `ActionEconomy.extra_action_used: bool`, serde default false, skipped when false, cleared when a turn starts like the other flags.

Engine (`bench-core/src/engine.rs`):
- A combatant has an extra action when an applying `ExtraAction` modifier is on it and `extra_action_used` is false.
- Taking an action as the extra action checks the use list. `one_attack`: the ability's effects are exactly one `Attack` with `target_count` 1, and no `Multiattack`, `Cast` or `UseAction`. Its usage limit applies as usual. `disengage`: the Disengage step. Anything else is `Illegal` ("<actor>'s extra action allows only <uses>"). Spending it sets `extra_action_used`, not `action_used`.
- `can_act` still gates it: an Incapacitated bearer can't use it.

Data (`data/spells/effects.json`): Haste gets a fourth modifier, `extra_action` with `only: [one_attack, disengage]`, concentration duration, `source_ability: "haste"`, and `on_end` holding the Incapacitated condition and a `set` 0 Speed modifier, both `UntilEndOfNextTurn` (bearer `target`). Put `on_end` on this modifier only, so lethargy lands once. Replace the two agent clauses with one: "The extra action can also Dash, Hide or Utilize." Check that `scripts/check-spell-effects.py` accepts it.

CLI and live (`bench/src/main.rs`, `bench-core/src/live.rs`):
- `bench action <actor> --action <id> [--target ...] --extra` and `bench disengage <actor> --extra` spend the extra action. Without `--extra`, both behave as today.
- `Command::Action` and `Command::Disengage` gain `extra: bool`, serde default false and skipped when false, so stored command histories replay unchanged.
- The digest shows an available extra action and its uses for the active combatant, for example `extra_action: ["one_attack", "disengage"]`, and nothing when there is none.
- `bench help --json` lists the flag. The skill's flag test reads it from there.

Batch (`bench-core/src/batch.rs`): no change here. Policy v2 never casts Haste, so no combatant gains an extra action in batch, and batch numbers don't move. Ticket 056 adds the policy's use.

Tests (write them first):
- A Hasted ally makes one weapon attack with `--extra` after its action. A second `--extra` the same turn is refused. A Multiattack or a spell with `--extra` is refused. `bench disengage --extra` sets the Disengage flag and leaves the action unused.
- Without Haste, `--extra` is refused.
- Breaking the caster's concentration ends Haste on the ally and leaves the ally Incapacitated with Speed 0. If that happens on the caster's turn, the ally stays Incapacitated through the end of its own next turn and recovers after it. If it happens on the ally's own turn, it lasts through the end of the ally's following turn.
- Two Hastes on one bearer give one extra action. Ending one Haste applies lethargy even though the other still applies (PHB: the target is Incapacitated "when the spell ends").
- A homebrew stat block whose bonus action grants an `extra_action` with empty `only` lets its bearer take a Multiattack as the extra action.
- Old states and histories: a state file saved before this build passes `store::check`, and `bench replay` on a stored history without `extra` reproduces its state hashes.
- Batch: a fixed-seed batch on goblin-warrior and one adult dragon at levels 5 and 11 gives the same results as `master`. Record it in the Resolution.

Version and baseline:
- Bump the workspace version, and with it `ENGINE_VERSION`, by one patch from what `master` holds when this lands. Don't regenerate `data/baseline/curve.json`; ticket 056 does that once.
- The engine stale reason is already accepted by `stale_beyond_the_engine` in `bench-core/tests/calibration.rs` and by `batch_compares_against_the_embedded_curve_outside_the_repo` in `bench/tests/cli.rs`.
- `a_fixture_regenerates_to_its_committed_row`: if the row differs and the test is not ignored yet, add `#[ignore = "curve stale until ticket 056 regenerates it"]`. Expect it not to differ. Don't edit the committed row. Ticket 056 removes the ignore.

Docs:
- Amend ADR 0005: the `extra_action` modifier target, `on_end`, `UntilEndOfNextTurn`, and the schema-version exception for these additive fields.
- Amend ADR 0004 or the live-mode reference: an extra action is spent with `--extra`.
- `.agents/skills/homebrew-bench/references/live-mode.md` (or whichever reference documents `bench action`): `--extra`. The homebrew conversion reference: "gains an additional action" maps to `extra_action`.
- `CHANGELOG.md` under Unreleased.

Hand-off to 056: the policy reads the bearer's extra action through the same check the engine uses, and the buff value of an `extra_action` modifier is the bearer's best `one_attack` value over the horizon.

## Resolution

Built on branch `ticket-066-build-haste-extra-action-and-lethargy`. A first builder was cut off mid-build; a second builder reviewed its uncommitted work against this ticket, found it complete, verified it and closed the ticket.

- Model. `ModifierTarget::ExtraAction { only: Vec<ExtraActionUse> }` (`one_attack`, `disengage`; empty means any action). `TimedModifier.on_end` and `ActiveModifier.on_end` are serde default and skipped when empty. `ActionEconomy.extra_action_used` is skipped when false and reset with the economy at turn start. Durations `until_end_of_next_turn` and `until_end_of_turn`: storage is the flag the ticket suggested, done by rewriting `until_end_of_next_turn` to `until_end_of_turn` when the bearer's next turn starts, and `until_end_of_turn` ends in `finish_turn` for that bearer. Old states serialize nothing new.
- Engine. `extra_action_uses(state, id)` (public) returns the joined uses of every applying `extra_action` modifier through `modifiers_on`, or `None` when there is none, it is spent, or the bearer can't act. `extra_action_allows(uses, action)` (public) is the use check: `one_attack` means effects are exactly one `Attack` with `target_count` 1. Ticket 056's policy should call these two. `resolve_action(.., extra)` and `disengage(.., extra)` refuse "<actor> has no extra action", "extra action already used", or "<actor>'s extra action allows only <uses>", log "<actor> uses its extra action", and set `extra_action_used`. Modifiers ended by duration, concentration, damage or a triggering attack go through `take_modifiers` and `fire_on_end`, which logs "<ability> ends on <bearer>" and resolves `on_end` from the source onto the bearer.
- Data. Haste's fourth modifier is `extra_action` with `on_end` holding Incapacitated and a `set` 0 Speed, both `until_end_of_next_turn` with `source_ability: "haste-lethargy"` so they don't stack-group with Haste. The single agent clause is "The extra action can also Dash, Hide or Utilize." The standard casters 05 to 20 carry the same clause. `check-spell-effects.py` renders `on_end` facts and reports 84 entries consistent.
- CLI and live. `--extra` on `bench action` and `bench disengage`, which `bench help --json` lists. `Command::Action`/`Disengage` and `StepRequest` carry `extra`, skipped when false. `digest.extra_action` shows the active combatant's uses (`["any"]` for an empty list), and only for party combatants in the public view.
- Tests (engine and live): one extra attack after the action, a second refused; Multiattack, spell and two-target attack refused; Disengage `--extra` leaves the action unused; no Haste refused; lethargy from the caster's turn and from the ally's own turn; two Hastes give one extra action and either ending brings lethargy; an empty `only` homebrew bonus action allows a Multiattack; Haste's `on_end` sits on one modifier; a live hasted striker uses `--extra` and replays matching.
- Old states. `bench-core/tests/data/engine-0.1.6-state.json` (a 0.1.6 encounter with a disengage and a Haste cast) loads through `store::load`, passes `store::check`, re-serializes unchanged, and replays its first four commands (no `extra`) with matching hashes, then diverges at the Haste cast because Haste now adds a modifier. The version gate already sends that case to `bench replay`.
- Versions. Workspace and `ENGINE_VERSION` 0.1.7; policy stays v3. `a_fixture_regenerates_to_its_committed_row` passes, so it isn't ignored. `cargo test --workspace` passes; clippy shows only master's two engine.rs errors (redundant clone at the triggering-attack loop, too many args on `apply_damage`). The SRD import exits 0 with 341 of 341 graduated.
- Batch numbers. Release builds of master and this branch, `bench batch <id> --levels <L> --fights 200 --seed 66` (goblin warrior with `--assume` for scimitar and shortbow): goblin warrior and adult red dragon at levels 5 and 11 give byte-identical output apart from the engine version strings.

Readings chosen where the ticket was open:

- `until_end_of_turn` is a second new duration, used for the stored form of `until_end_of_next_turn` and available to homebrew ("until the end of its turn").
- `on_end` does not fire when an override clears a modifier, or when the encounter ends.
- The lethargy is not removed early when another Haste still applies; the PHB ties it to "when the spell ends".
- An extra action can't take a bonus action or reaction entry ("..., which is not an action").

No follow-up tickets or new fog.
