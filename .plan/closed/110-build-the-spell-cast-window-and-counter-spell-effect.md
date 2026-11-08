---
id: 110
title: Build the spell-cast window and counter_spell effect
type: task
mode: AFK
status: closed
blocked_by: [101]
claimed: false
---

## Question

Build the engine half of [Counterspell and the spell-cast trigger](../closed/101-counterspell-and-the-spell-cast-trigger.md): the `spell_cast` trigger, the `counter_spell` effect, the window, and the gate.

## Notes

- Skills: `implement`, `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- Design: [ADR 0019](../../docs/adr/0019-counterspell-through-a-spell-cast-window.md). Follow ADR 0018's hold-and-rerun pattern.
- `trigger: {spell_cast: {caster: enemy}}`, refused by the loader outside a reaction. A leveled cast by an enemy of the reactor is held before slot, usage and effects resolve. Cantrips, allies' casts and reaction casts open no window.
- `Effect::CounterSpell`, additive at schema version 1: Constitution save against the reactor's spell save DC, `spell_countered` event on failure, slot not expended, action and per-day usage stay spent.
- Gate: `settings.spell_cast_windows`, set by `bench start` and batch, false for older saves. Bump the engine version. Test `undo` and `replay` on an old save.
- Log lines and public view as in ADR 0019. Update ADR 0005 with the third trigger and the effect.
- Live mode: update the skill's reaction step so the agent answers a spell-cast window with `bench respond`.
- Batch stays on its current policy until [Build batch Counterspell and the Protective Magic split](111-build-batch-counterspell-and-the-protective-magic-split.md); the pass-all decider declines.

## Resolution

Built in auto mode on 2026-11-08 as the notes say, with these choices.

- **Where the window sits.** `cast_spell` asks the spell-cast windows after the cast level is known and before the slot is counted, so a countered cast never expends it. The caster's action and any per-day usage are spent earlier, in `resolve_action`, and stay spent. A window that waits, or a counter whose save waits on a player's die, ends the run so the caller discards it and prompts, as ADR 0018 does for hits.
- **No window for a reaction cast.** `Reactions.reacting` is set while `resolve_action` runs a reaction, so Counterspell and Shield open no window.
- **The effect.** `counter_spell` is a struct variant with a `dc` (0 takes the reactor's spell save DC, bound like a saving throw's DC). It rolls the caster's Constitution save through a new `save_succeeds` helper that `resolve_save` now shares, and a failure logs `spell_countered` and sets `Reactions.countered`.
- **Batch.** Batch sets the gate, and the policy passes every spell-cast window without a reason, so logs and numbers don't change until ticket 111. The curve differs only in its engine version.
- **Data.** Counterspell in the effects file carries `counter_spell` and drops the save and slot clauses. The fixtures still hold Counterspell as an agent clause.
- **Live.** A window message reads `<reactor>: <caster> is casting <spell> (level <n>), take <id>?`, and an answer joins the queued answers as a hit answer does. The skill's live-mode reaction step and ADR 0005 are updated.

Engine 0.1.28, policy stays v4.
