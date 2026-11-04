---
id: 059
title: Build upcast spell scaling
type: task
mode: AFK
status: open
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
  - Heal: dice `1d1` with modifier 69, and `extra_modifier` 10.
  - Acid Arrow and Vitriolic Sphere: drop the rules that point at a missing effect, and move the unmodeled parts into `upcast_clause`.
  - Bless (no effects): a clause in place of the rule.
  - Ice Knife: the cold burst is unmodeled, so its 1d6 becomes a clause and not a rule on the piercing attack.
  - Ice Storm: `damage_type: bludgeoning`.
  - Arcane Hand and Wall of Ice: a clause for the part not modeled.
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
- Engine: Fireball at level 5 rolls 10d6. Magic Missile at level 3 makes five darts. Scorching Ray at level 3 makes four rays. Cure Wounds at level 2 heals 4d8 plus the modifier. Heal at level 7 heals 80. An `extra_targets` rule on a test entry's Attack raises its `target_count` at a higher level. Hold Monster at level 6 still resolves against every target passed. A slot level below the spell's level is refused. A cantrip's damage still scales with caster level, and a leveled spell with no slot level does not. An `upcast_clause` event appears only on an upcast.
- Live: `--slot` spends the named slot and scales the spell. Each refusal gives its reason. A history without `--slot` replays.
- Batch: the clause warning appears for a homebrew stat block that upcasts Acid Arrow.

Docs:

- Amend ADR 0014: cast level, `level`, the four rule variants, `upcast_clause`, and the retired variants. A new ADR is not needed.
- Document `--slot` in `.agents/skills/homebrew-bench/references/live-mode.md`. The skill test checks it against `bench help --json`. Say in the live and batch references what an `upcast_clause` line means for the agent.
- Add a `CHANGELOG.md` entry. It must say that the 14 dragon fixtures that cast "level N version" spells now cast them at that level, and that Magic Missile and Heal now resolve.

Versions and baseline:

- Bump the workspace version, and with it `ENGINE_VERSION`, to 0.1.2. Do not regenerate `data/baseline/curve.json`. The batch numbers for these dragons change, and caster cells may change if Magic Missile or Heal was being picked. Before the fix, check whether those casts errored or were never chosen, and record which in the Resolution.
- The stale-curve allowances in `bench-core/tests/calibration.rs` and the CLI test already accept any "the engine is ..." reason, so they should pass unchanged. Ticket [056](056-build-v3-scoring-for-buffs-forced-movement-and-teleports.md) waits on this ticket and regenerates the curve once after it lands.

Consult `tdd`, `codebase-design`, `domain-modeling` and `code-review`.

## Resolution
