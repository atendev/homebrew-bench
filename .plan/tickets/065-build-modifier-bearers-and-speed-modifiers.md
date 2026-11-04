---
id: 065
title: Build modifier bearers and Speed modifiers
type: task
mode: AFK
status: open
blocked_by: [055]
claimed: false
---

## Question

Build decisions 1 to 5 of [Haste extra action and ally-targeted buffs](../closed/055-haste-extra-action-and-ally-targeted-buffs.md): a timed modifier lands on a bearer apart from its source, the same spell doesn't stack on one bearer, a modifier can end on damage, and Speed modifiers are read. The batch policy's bearer choice (decision 6) is built by ticket 056, not here.

## Notes

Consult `tdd`, `codebase-design`, `domain-modeling` and `code-review`. Read ADR 0005 first.

The fault being fixed: `resolve_effect` (`bench-core/src/engine.rs`, the `Effect::TimedModifier` arm) pushes the modifier with `source` set to the actor and ignores `target_ids`. `attack_modifier`, `save_modifier`, `ac_modifier`, `apply_modifier_target_disposition` and the Reactions check in `opportunity_eligibility` read `source` as the creature that has the modifier. So Slow's and Hypnotic Pattern's `on_failure` modifiers land on the caster. Nothing reads `ModifierTarget::Speed`: `engine.rs` (the move-then-attack arm and `move_combatant`), `tactics.rs` (`movement_left`) and `live.rs` (`remaining_speed`) each read the stat block's walk speed.

Model (`bench-core/src/effect.rs`, `bench-core/src/encounter.rs`):
- `TimedModifier` gains `bearer: ModifierBearer` (`target` or `self`, serde default `target`) and `ends_on_damage: bool` (serde default false, skipped when false, as on `ConditionEffect`).
- `ActiveModifier` gains `bearer: Option<String>` (absent means the bearer is `source`), `source_ability: Option<String>` and `ends_on_damage: bool`. The engine sets `bearer` only when it differs from `source`, and every reader goes through one accessor that returns `bearer` or else `source`. Serialize none of the three when absent or false (`skip_serializing_if`), and give each a serde default.
- Saved states must keep their hashes. `store::check` compares the stored tamper hash and the log's last `state_hash` with hashes of the re-serialized state (`bench-core/src/store.rs`, `check`), so a field that serializes on an old state turns every saved encounter with an active modifier into `StateTampered`. The skip rules above prevent that: an old modifier, and any self-borne modifier with no `source_ability`, serializes exactly as before.
- These are additive fields with defaults. Keep the schema at version 1 and record the exception in ADR 0005, as ticket 054 did for `no_provoke`. Regenerate the JSON schema files if the repo commits them.

Engine (`bench-core/src/engine.rs`):
- `self` modifiers land on the actor. `target` modifiers land on each id in `target_ids`. A top-level `target` modifier resolved with no target is `EngineError::Illegal` ("<ability> needs a target for its modifier"). Nested under `on_failure` or `on_hit`, the target list is already the one target.
- `source` stays the actor. `break_concentration` keeps removing modifiers by `source`.
- Every reader keys on `bearer`: `attack_modifier`, `save_modifier`, `ac_modifier`, `apply_modifier_target_disposition` and the Reactions check in `opportunity_eligibility`. `RollModifier` pushes with `bearer` = the actor.
- Same spell on the same bearer (PHB "Combining Spell Effects", p. 236): keep every instance. When reading, modifiers on one bearer with the same `source_ability` (when set) and the same `target` form a group, and only one of them applies: the most potent, else the most recent. Potency: for `add`, the larger value; for `set` on Speed, the lower value; every other change counts as equally potent. "Most recent" is the later one in `active_modifiers`. When the applying instance ends, the next winner applies on its own. Modifiers with no `source_ability` always apply. Put the rule in the accessor that every reader and the walk-speed function use.
- `ends_on_damage`: remove the bearer's modifiers that have it when the bearer takes damage, in the same place conditions with `ends_on_damage` end.
- One walk-speed function, crate-visible, used by the engine, `tactics.rs` and `live.rs` in place of their three copies: the lowest `set` wins outright; otherwise walk speed plus the sum of `add`, then doubled once per `double` and halved once per `halve`, rounding down, floored at 0. Grappled and Restrained give 0 as they do now. Only `ModifierTarget::Speed` modifiers on that bearer count.
- The modifier event names the bearer: "<bearer> gains <source_ability or 'a timed modifier'> from <source>".

Data (`data/spells/effects.json`, `scripts/`):
- Shield: `"bearer": "self"`. Haste, Slow and Hypnotic Pattern stay on the default `target`. Hypnotic Pattern's Speed 0 gets `"ends_on_damage": true`.
- Fly: the file models it as a `double` on Speed, but the 2024 spell gives the target a Fly Speed of 60 feet (PHB p. 275). `ModifierTarget::Speed` has no movement mode, and the engine moves by walk speed only, so a fly-speed modifier would be inert. Remove the modifier and its `extra_targets` scaling rule, keep "Target gains a Fly Speed of 60 feet and can hover for the duration" as an agent clause, and add "One additional target per spell slot level above 3" as an agent clause (or an `upcast_clause` if ticket 059 has landed). Fly then has no effects, like Bless; check that the cast still resolves and logs its clauses.
- `scripts/scaffold-spells.py` writes `bearer: self` for a spell whose SRD range is Self, and `scripts/check-spell-effects.py` accepts the new fields.
- Other modifiers in the file: check each top-level one and set `self` where the SRD range is Self.

Live (`bench-core/src/live.rs`): `bench action <caster> --action <spellcasting> --target <ally>` casting Haste puts it on the ally. Check that live mode lets a caster target an ally or itself with a cast; if it refuses, allow it for casts whose effects include a top-level `target` modifier.

Tests (write them first):
- Slow's failed save puts -2 AC, -2 Dexterity saves and no reactions on the target, and nothing on the caster.
- Haste cast on an ally gives that ally +2 AC, Advantage on Dexterity saves and double Speed, and the caster none of it. Breaking the caster's concentration removes all three from the ally.
- Shield still gives its caster +5 AC on its own.
- A top-level `target` modifier with no target is refused.
- Two Hastes from two casters on one bearer: both sets are stored, the bearer gets +2 AC once, not +4, and doubled Speed once. Ending either caster's concentration leaves the other set applying.
- Potency: a +1 and then a +3 AC from the same `source_ability` on one bearer give +3; ending the +3 brings back the +1. Two equal ones apply once.
- Hypnotic Pattern's Speed 0 ends when the target takes damage, together with its conditions.
- Walk speed: halved 30 is 15, doubled 30 is 60, halved and doubled 30 is 30, `set` 0 beats a `double`, an `add` of -40 on 30 is 0. A Hasted creature can make a 60-foot move in live and in the engine, and `movement_left` in the policy agrees.
- Old states: an encounter state file saved before this build, with an active modifier (write it with `master`'s binary or take a committed fixture state), passes `store::check` and loads with each modifier's bearer read as its `source`.
- A modifier whose bearer is its source serializes with no `bearer` key, and one without `ends_on_damage` has no `ends_on_damage` key.

Version and baseline:
- Bump the workspace version, and with it `ENGINE_VERSION`, by one patch from what `master` holds when this lands. 059, 061 and 062 also take the next patch in landing order. Don't regenerate `data/baseline/curve.json`; ticket 056 does that once.
- Batch numbers should not move under policy v2: v2 scores every `TimedModifier` at 0 and never casts a buff, Shield is self-cast, and Hypnotic Pattern's targets are Incapacitated and skip their turns. Check it from a scratch run outside the repo: a fixed-seed batch on goblin-warrior, ogre and one adult dragon at levels 1, 5 and 11 against `master`. Record the result in the Resolution. If a number moves, find the event behind it in the log before going on, and record it.
- The engine stale reason is already accepted by `stale_beyond_the_engine` in `bench-core/tests/calibration.rs` and by `batch_compares_against_the_embedded_curve_outside_the_repo` in `bench/tests/cli.rs`.
- `a_fixture_regenerates_to_its_committed_row` regenerates goblin-warrior and compares it with the committed row. If the row differs and the test is not ignored yet, add `#[ignore = "curve stale until ticket 056 regenerates it"]`. Don't edit the committed row. Ticket 056 removes the ignore.

Docs:
- Amend ADR 0005: a consequence bullet on bearer and source, `self` versus `target`, no stacking of the same spell on one bearer, `ends_on_damage` on modifiers, the Speed rule, and the schema-version exception.
- `.agents/skills/homebrew-bench/references/` (whichever file explains the effect vocabulary for converting homebrew): a self-buff needs `"bearer": "self"`; a modifier is otherwise on its target.
- `CHANGELOG.md` under Unreleased: modifiers land on their target, Speed modifiers work, and the curve reads stale until it is regenerated.

Overlaps: 061 rewrites `apply_modifier_target_disposition` into the two-flag helper, and 059 and 062 edit `effects.json` and casting. Whichever lands second rebases. Hand-off to 056: it reads `bearer` and `source_ability` on `ActiveModifier` for the buff plan's bearer choice and its "already active" check.

## Resolution
