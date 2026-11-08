---
id: 110
title: Build the spell-cast window and counter_spell effect
type: task
mode: AFK
status: open
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

<Filled in at close.>
