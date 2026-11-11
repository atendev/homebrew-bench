---
id: 007
title: Live start with homebrew spells
type: task
mode: AFK
status: closed
blocked_by: [006]
claimed: false
---

## Question

Build the start-time rules from [Live spell casting prompts](../closed/003-live-spell-casting-prompts.md) ([ADR 0022](../../docs/adr/0022-live-spell-casts-reuse-bench-action.md)): `bench start --spell <file>` copies homebrew entries into the encounter and checks every cast that names one.

## Notes

- Skills: `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- `bench start` takes repeatable `--spell <file>`, loads through the spell book from [Spell book](006-spell-book.md), and copies each entry into state with its fingerprint.
- Refuse a stat block when a homebrew cast sits in a section that doesn't match its `casting_time` (`actions` = `action`, `bonus_actions` = `bonus_action`, `reactions` = `reaction`), sits in `traits`, `legendary_actions` or any other section, or names a `long` entry. The message names the action, its section and the casting time.
- Refuse a leveled homebrew save or attack spell cast from a stat block with no `spellcasting` block. A utility spell needs none.
- A cast naming an id no loaded file supplies keeps the ADR 0005 rule (`unmodeled_cast`), and `bench start` prints a warning naming the action and the id.
- `--spell` on any command against a running encounter is refused with a pointer to `bench start`.
- A homebrew cast through `bench action` resolves as an SRD cast does: the spell-cast window opens for a leveled action or bonus action cast, not for a cantrip or a reaction cast (ADR 0019).
- Slot: a monster casts at its stat block's `slot_level`, and `--slot` is refused for a monster. A PC's `--slot` above the entry's level takes effect only when the effect's `upcast` is true.
- Tests: one per refusal; monster `--slot` refused; PC upcast with and without `upcast: true`; the warning; a PC and a monster cast of a homebrew entry; replay of a saved encounter with a homebrew entry needs no file.

## Resolution

Built in engine 0.1.38. `bench start` takes repeatable `--spell <file>`, loads it through the spell book loader, copies each entry into state, and writes a hidden `SPELL` line with the entry's fingerprint into log entry 0. `stat_block::check_live_casts` refuses a homebrew cast whose section doesn't match its `casting_time`, a cast in `traits` or `legendary_actions`, a `long` entry, and a leveled attack or save spell on a stat block with no `spellcasting` block. Each message names the stat block, action, section and casting time. `bench add` runs the same checks. A cast of an id the book lacks starts with a `warnings` entry naming the action and the id. `--spell` on a running encounter is refused with a pointer to `bench start`. `--slot` is refused for a monster. On a PC's homebrew cast, a slot above the entry's level is refused unless the cast has `upcast: true`, and SRD casts keep their old rule. The spell-cast window needed no change. Decisions are in [the decision report](../reports/007-decisions.md). Haiku audit: APPROVE in round 2.
