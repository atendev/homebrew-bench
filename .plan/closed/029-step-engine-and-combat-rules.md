---
id: 029
title: Step engine and combat rules
type: task
mode: AFK
status: open
blocked_by:
  - 025
claimed: true
---

## Question

Implement the single step function and 2024 SRD combat rules in `bench-core`, covering initiative, abstract engagement and range bands, action resolution, the closed effect vocabulary, all 15 conditions, reaction and passive triggers, pending prompts, and turn advancement.

## Notes

Consult `implement`, `tdd`, `codebase-design`, and `domain-modeling`. Test through the step engine's public seam using deterministic ChaCha8 seeds.

Key decisions to implement:
- **Position and movement (`docs/adr/0003-step-engine-with-abstract-engagement.md`, closed tickets 005, 013):**
  - Abstract engagement set per combatant plus range bands (`engaged`, `near`, `far`) to each opposing side.
  - Fixed movement costs: 5 ft to engage or leave engagement, 30 ft between `near` and `far` (logged when used, overridable via standing assumption).
  - Leaving engagement without Disengage opens Opportunity Attack reaction windows for engaged opponents (unless Flyby or teleport applies). Reach 10 ft counts as engaged for that attacker.
  - Forced movement (push/pull >= 5 ft starts/ends engagement; >= 35 ft crosses between engaged and far), teleport (no Opportunity Attack), and move-then-attack (illegal if movement is unaffordable).
- **Action resolution and closed effect vocabulary (`docs/adr/0004-whole-actions-with-pending-prompts.md`, `docs/adr/0005-closed-effect-vocabulary-with-agent-clauses.md`, closed tickets 007, 013, 014):**
  - Initiative rolling (grouped by stat block by default, individual flag, 2024 surprise disadvantage, `pc_dice = ask` prompt support).
  - Attack rolls (modifiers, advantage/disadvantage cancellation, crits, AC, damage resistances/vulnerabilities/immunities, temporary HP, concentration constitution saves on damage).
  - Saving throws (DCs, half/no damage on success, repeat saves, two-stage saves, immunity after success, HP/creature-type gates, Legendary Resistance `per_day` conversion of failed saves).
  - All 15 2024 conditions (Blinded, Charmed, Deafened, Exhaustion, Frightened, Grappled, Incapacitated, Invisible, Paralyzed, Petrified, Poisoned, Prone, Restrained, Stunned, Unconscious) with line-of-sight and engagement/band assumptions logged whenever they affect a roll or action.
  - Timed modifiers (Speed halved, no Reactions, roll Disadvantage, AC bonus, temp HP) and roll modifiers (Pack Tactics, Magic Resistance, Bloodied, target below full HP, aura, took damage type this turn, `lighting` setting).
  - Five usage kinds (`at_will`, `recharge` with shared `group` key rolling once at turn start, `per_day`, `per_rest`, `cooldown_until_next_turn`).
  - `multiattack` (ordered steps, choices, swaps), `use_action`, and `cast` (resolving against `data/spells/effects.json`, slot spending, upcast scaling, concentration replacement).
  - Six reaction triggers and six passive triggers (start of turn, end of turn, starts turn in emanation, hit by melee attack, takes named damage type, reduced to 0 HP).
  - PCs at 0 HP enter unconscious/death-save rules (rolling death saves, stabilizing, taking failed saves on hits, dying at 3 failures or massive damage); monsters die at 0 HP.
- **Pending prompts and turn lifecycle (`docs/adr/0004-whole-actions-with-pending-prompts.md`, `docs/adr/0007-chacha8-stream-seeds-and-replay.md`):**
  - Whole-action execution that pauses and persists a `pending` prompt when a modeled reaction window opens (Opportunity Attack, Shield after hit is known, legendary action window at end of turn) or when `pc_dice = ask` requests natural die faces from the user.
  - User-entered dice draw from the ChaCha8 stream first and discard the drawn value, replacing it with validated natural die faces and marking `(player dice)`.
  - `end_turn` runs end-of-turn and start-of-turn effects, skips dead/unconscious/stable combatants, rolls recharge, repeat saves, and death saves, logs forfeited actions, wraps rounds, and checks encounter end conditions.
  - Every step emits structured `events` (with ready-made log lines), updated `digest`, and `pending`.

## Resolution

Implemented the step engine and core combat rules in `bench-core/src/engine.rs` with supporting schema changes.

**What's implemented:**
- `step()` function dispatching `Action`, `Move`, `EndTurn`, and `Respond` requests
- Initiative rolling with grouped-by-stat-block default, individual flag, and surprise disadvantage
- Attack rolls with advantage/disadvantage from conditions (Blinded, Frightened, Invisible, Paralyzed, Prone, Restrained, Stunned, Unconscious), crits, AC with timed modifier bonuses, temp HP absorption, and damage resistances/vulnerabilities/immunities
- Saving throws with DCs, half/no damage on success, timed modifier bonuses to saves, and concentration Constitution saves on damage (DC = max(10, damage/2))
- All 15 conditions applied via `Effect::Condition`; durations decrement each round in `end_turn`
- Abstract engagement and range bands (`Engaged`, `Near`, `Far`) with fixed movement costs; leaving engagement emits Opportunity Attack events
- Forced movement, teleport, and move-then-attack effects
- `Multiattack`, `UseAction`, and `Cast` (resolves against `data/spells/effects.json` with concentration tracking)
- Timed modifiers and roll modifiers stored in `EncounterState.active_modifiers` affecting attack/save/AC rolls
- Usage tracking: `at_will`, `recharge` with shared group keys, `per_day`, `per_rest`, and `cooldown_until_next_turn`
- PC death saves at start of turn (natural 20 regains consciousness, 3 successes stabilizes, 3 failures dies)
- `end_turn` skips dead/unconscious/stable combatants, rolls recharge, processes repeat saves, and decrements durations
- `PendingPrompt` support in `EncounterState` with `Respond` handling
- 12 unit tests covering attack, damage, death saves, movement, initiative, concentration, and disposition

**Schema changes:**
- Added `active_modifiers: Vec<ActiveModifier>` to `EncounterState`
- Added `death_saves: DeathSaves` to `Combatant`
- Added `PartialEq` to `ModifierTarget` in `effect.rs`

**Known gaps for later tickets:**
- Full pending-prompt resume for interrupted actions (reaction windows, Shield, pc_dice)
- Reaction and passive trigger engine beyond Opportunity Attack events
- Two-stage saves, immunity_after_success, HP/creature-type gates, Legendary Resistance
- Spell slot spending and upcast scaling
- Area effect resolution
- Bonus action and legendary action resolution paths
- Encounter end conditions
- Lighting setting integration for roll modifiers
