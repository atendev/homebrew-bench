---
id: 059
title: Build upcast spell scaling
type: task
mode: AFK
status: closed
blocked_by: [050]
claimed: false
---

## Question

Build what [Upcast spell scaling](../closed/050-upcast-spell-scaling.md) decided: a cast resolves at its cast level, the effects file's scaling rules apply once per level above the spell's own level, unmodelable upcast text is kept as an `upcast_clause`, and live mode can pick a slot with `bench action --slot`.

## Notes

Model and file (`bench-core/src/spell.rs`, `data/spells/effects.json`):

- Add a required `level: u8` to `SpellEffect`. Add an optional `upcast_clause: Option<String>`.
- `Scaling` becomes `ExtraDice { dice, damage_type: Option<DamageType> }`, `ExtraModifier { amount: i32 }`, `ExtraTargets { count }` and `ExtraEffects { count }`. Remove `IncreaseDuration` and `AddDamageType`.
- Fill `level` on all 84 entries from the SRD. Move the "Upcast:" text in `notes` (Aid, Heal, Hold Person and others) into `upcast_clause` or a rule.
- Fix the known faulty entries:
  - Magic Missile: dice `1d4` with modifier 1, and one `extra_effects` rule instead of three `extra_dice` rules.
  - Scorching Ray: one `extra_effects` rule instead of three `extra_targets` rules.
  - Heal: a flat 70 with no roll and no spellcasting modifier, and `extra_modifier` 10. `bind_effect` adds the caster's modifier to every Heal, and `1d1` would prompt a live player for a d1, so `1d1 + 69` is wrong. Make `Heal.dice` optional (absent means no roll) and add a Heal field, such as `spellcasting_modifier: bool` defaulting to true, that `bind_effect` reads. Heal sets it to false.
  - Disintegrate, Finger of Death and Regenerate: move the flat part into `modifier` (`10d6` + 40, `7d8` + 30, `4d8` + 15).
  - Acid Arrow and Vitriolic Sphere: drop the rules that point at a missing effect, and move the unmodeled parts into `upcast_clause`.
  - Bless, Wall of Fire and Wall of Ice (no effects): a clause in place of each rule.
  - Ice Knife: the cold burst is unmodeled, so its 1d6 becomes a clause and not a rule on the piercing attack.
  - Ice Storm: `damage_type: bludgeoning`.
  - Arcane Hand: a clause for the Grasping Hand part.
  - Shatter: add the missing `extra_dice 1d8` rule.
- Check every other rule against the SRD text while doing this.

Engine (`bench-core/src/engine.rs`):

- Add one function that turns a spell entry and a cast level into the effects to resolve. It applies each rule `cast_level - level` times. `resolve_cast` and `bound_cast_effects` both call it before `bind_effect`, so the batch policy scores the scaled spell.
- The cast level is `slot_level`, or `level` when it is empty. A slot level below `level`, or any slot level on a cantrip, is `EngineError::Invalid`.
- A cantrip is `level == 0`. It is no longer spotted by `slot_level`. `cast_context` and `bind_effect` read it from the entry.
- `spend_slot` and the slot resource counter use the cast level.
- An upcast of an entry with an `upcast_clause` pushes an `upcast_clause` event: `<actor> casts <spell> at level <n>; agent clause: <text>`. It goes in the log and the public view like other agent-clause lines.

Live (`bench/src`, `bench-core/src/live.rs`):

- Add `bench action --slot <level>`. It sets the cast level of every Cast in the action. Refuse it, with the reason, when the stat block has no `spell_slots`, the action has no leveled Cast, the level is below a spell's level, or no slot of that level is left.
- The flag is stored in the command history. An old `Action` command without it must still deserialize and replay.

Batch (`bench-core/src/batch.rs`):

- Next to the missing-spell warning, warn once per upcast Cast in the monster's stat block whose entry has an `upcast_clause`: `Spell <id> at level <n>: <clause> is not applied; results may understate this monster.`
- Batch passes no slot choice. `POLICY_VERSION` stays at 2.

Scripts:

- `scripts/scaffold-spells.py` emits `level`. It reads the higher-level text from `higher_level` and from the description's "Using a Higher-Level Spell Slot." paragraph, and ignores "Cantrip Upgrade.". It writes schema-valid rules: "increases by NdM" becomes `extra_dice` (with `damage_type` when the text names a type), "increases by N" becomes `extra_modifier`, one more dart, ray or beam becomes `extra_effects`, and one more target, creature or bolt becomes `extra_targets`. Anything else becomes `upcast_clause`. Fix the string-shaped `extra_dice` and `extra_targets` it writes today.
- `scripts/check-spell-effects.py` checks that `level` matches the SRD, that every spell with higher-level text has a rule or an `upcast_clause`, and that each rule's dice or amount appears in that text.

Tests (`tdd`):

- In `spell.rs`: each rule's `effect_index` exists, the variant fits that effect, `extra_dice` die sizes match their target, and every dice string in the file parses with `roll_dice`'s grammar.
- Over fixtures and the standard party: `upcast` is true exactly when `slot_level` is above the spell's `level`, and no cast has a slot level below its spell's level.
- Engine: Fireball at level 5 rolls 10d6. Magic Missile at level 3 makes five darts. Scorching Ray at level 3 makes four rays. Cure Wounds at level 2 heals 4d8 plus the modifier. Heal at level 7 heals exactly 80: no spellcasting modifier, no dice drawn, and no player-dice prompt in ask mode. Disintegrate deals `10d6` + 40. An `extra_targets` rule on a test entry's Attack raises its `target_count` at a higher level. Hold Monster at level 6 still resolves against every target passed. A slot level below the spell's level is refused. A cantrip's damage still scales with caster level, and a leveled spell with no slot level does not. An `upcast_clause` event appears only on an upcast.
- Live: `--slot` spends the named slot and scales the spell. Each refusal gives its reason. A history without `--slot` replays.
- Batch: the clause warning appears for a homebrew stat block that upcasts Acid Arrow.

Docs:

- Amend ADR 0014: cast level, `level`, the four rule variants, `upcast_clause`, and the retired variants. A new ADR is not needed.
- Amend ADR 0005: the multiattack bullet says `resolve_cast` ignores `slot_level` and `upcast` "until spell scaling lands". Replace that with the cast-level rule, and mention the optional Heal dice and the flag that turns off the spellcasting modifier.
- Document `--slot` in `.agents/skills/homebrew-bench/references/live-mode.md`. The skill test checks it against `bench help --json`. Say in the live and batch references what an `upcast_clause` line means for the agent.
- Add a `CHANGELOG.md` entry. It must say that the 14 dragon fixtures that cast "level N version" spells now cast them at that level, and that Magic Missile, Heal, Disintegrate, Finger of Death and Regenerate now resolve.

Versions and baseline:

- Bump the workspace version, and with it `ENGINE_VERSION`, to 0.1.2, or to 0.1.3 if ticket [061](061-fix-advantage-disadvantage-cancellation.md) has already taken 0.1.2. Do not regenerate `data/baseline/curve.json`. The batch numbers for these dragons change. Caster and healer cells may change if Magic Missile, Heal, Disintegrate, Finger of Death or Regenerate was being picked. Before the fix, check for each of the five whether its casts errored or were never chosen, and record which in the Resolution.
- Regenerate the standard party with `scripts/generate-party.py`. The generator copies `agent_clauses` from the effects file, so moving upcast text out of them (Magic Missile, Scorching Ray, Wall of Fire, Wall of Ice) changes 20 party files, and `committed_blocks_match_generator_output` in `party.rs` fails until they are regenerated. The curve then also reports "the party differs from the `standard` party the curve was computed against".
- Widen both stale-curve allowances, `stale_beyond_the_engine` in `bench-core/tests/calibration.rs` and the CLI test `batch_compares_against_the_embedded_curve_outside_the_repo`, to accept that party reason as well as the engine and fixture reasons. Ticket [056](056-build-v3-scoring-for-buffs-forced-movement-and-teleports.md) waits on this ticket, regenerates the curve once after it lands, and removes all three allowances.
  If ticket [062](062-build-live-reaction-windows.md) landed first, it has already added the party reason, so skip this step.
- `a_fixture_regenerates_to_its_committed_row` in `bench-core/tests/calibration.rs` regenerates goblin-warrior and compares it with the committed curve row. If the row differs and the test is not ignored yet, add `#[ignore = "curve stale until ticket 056 regenerates it"]`. Don't edit the committed row. Ticket 056 removes the ignore.

Consult `tdd`, `codebase-design`, `domain-modeling` and `code-review`.

## Resolution

Built on branch `ticket-059-build-upcast-spell-scaling`.

- Model. `SpellEffect` has a required `level` and an optional `upcast_clause`. `Scaling` is `ExtraDice { dice, damage_type? }`, `ExtraModifier { amount }`, `ExtraTargets { count }` and `ExtraEffects { count }`; `IncreaseDuration` and `AddDamageType` are gone. `Heal.dice` is optional and `Heal.spellcasting_modifier` defaults to true. `effect::parse_dice` is the one `NdM` grammar; `roll_dice` and the spell tests use it.
- Effects file. All 84 entries carry `level` from the SRD. The listed faults are fixed. Checking every rule against the SRD also found Prayer of Healing and Regenerate adding the spellcasting modifier, which the SRD text doesn't give, so both set it to false. Every "Upcast:" note and every higher-slot sentence in `agent_clauses` moved to a rule or an `upcast_clause`. That covers Aid, Bless, Dominate Monster and Person, Etherealness, Globe of Invulnerability, Mass Suggestion, Magic Missile, Scorching Ray, Wall of Fire and Wall of Ice. Hold Person's note went because its rule records it. Command keeps its clause, since it tells the agent to pass the extra targets. Vitriolic Sphere has no clause: only its initial damage scales, and its rule covers that.
- Engine. `cast_level` and `scaled_effects` in `engine.rs`. `cast_spell` (the `resolve_cast` of the ticket) and `bound_cast_effects` both call them before `bind_effect`. Copies from `extra_effects` are appended after the other rules apply, so they carry the scaled numbers. `spend_slot`, the slot counter and `ability_ready` use the cast level. A cantrip is `level == 0`. A dice-size mismatch at run time is `EngineError::Invalid`.
- Live. `StepRequest::Action` has an optional `slot_level`, set from `Command::Action { slot }` and `bench action --slot`. Both fields are skipped when empty, so old histories deserialize and replay.
- Batch. `engine::unapplied_upcasts` feeds the warning. `missing_spells` and it share one cast walker. `POLICY_VERSION` stays 2.
- The generator now sets `upcast` when the party's tier is above the spell's level. That affects 12 Dominate Person and Dominate Monster casts, which the new fixture-and-party test caught. With the clause moves, 38 party files changed, not 20. Both stale-curve allowances accept the party reason.
- The workspace is 0.1.3. `a_fixture_regenerates_to_its_committed_row` still passes (the goblin-warrior row has no casts), so it is not ignored. `cargo test --workspace` passes. Clippy shows the four warnings already on master. The import exits 0 with 341 of 341 graduated. `check-spell-effects.py` passes.
- Before the fix, in 800 sampled master fights (seed 11, six monsters, levels 1 to 20): batch's Heal casts errored (`invalid dice expression 70`, plan recomputed). Magic Missile, Disintegrate and Finger of Death were never chosen, because `dice_average` scored their dice strings 0. No standard party member casts Regenerate.

Readings chosen where the ticket was open:

- `--slot` sets only the action's top-level Casts. Party spell actions hold one Cast each, and multiattack casts are monster-only, where the MM forbids upcasting.
- `--slot` refusals are `EngineError::Invalid`, except "no slot left", which stays `Illegal` like an ordinary spent slot.
- The batch warning comes once per distinct spell and cast level, so a cast that appears in both a Spellcasting action and a multiattack option is named once. The clause loses its final period so the sentence reads.
- `extra_targets` and `extra_effects` fit any effect in the Rust rule test. `extra_modifier` must reach Heal, TempHp or a damage entry.

