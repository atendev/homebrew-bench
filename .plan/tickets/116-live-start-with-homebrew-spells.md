---
id: 116
title: Live start with homebrew spells
type: task
mode: AFK
status: open
blocked_by: [115]
claimed: false
---

## Question

Build the start-time rules from [Live spell casting prompts](../closed/003-live-spell-casting-prompts.md) ([ADR 0022](../../docs/adr/0022-live-spell-casts-reuse-bench-action.md)): `bench start --spell <file>` copies homebrew entries into the encounter and checks every cast that names one.

## Notes

- Skills: `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- `bench start` takes repeatable `--spell <file>`, loads through the spell book from [Spell book](115-spell-book.md), and copies each entry into state with its fingerprint.
- Refuse a stat block when a homebrew cast sits in a section that doesn't match its `casting_time` (`actions` = `action`, `bonus_actions` = `bonus_action`, `reactions` = `reaction`), sits in `traits`, `legendary_actions` or any other section, or names a `long` entry. The message names the action, its section and the casting time.
- Refuse a leveled homebrew save or attack spell cast from a stat block with no `spellcasting` block. A utility spell needs none.
- A cast naming an id no loaded file supplies keeps the ADR 0005 rule (`unmodeled_cast`), and `bench start` prints a warning naming the action and the id.
- `--spell` on any command against a running encounter is refused with a pointer to `bench start`.
- A homebrew cast through `bench action` resolves as an SRD cast does: the spell-cast window opens for a leveled action or bonus action cast, not for a cantrip or a reaction cast (ADR 0019).
- Slot: a monster casts at its stat block's `slot_level`, and `--slot` is refused for a monster. A PC's `--slot` above the entry's level takes effect only when the effect's `upcast` is true.
- Tests: one per refusal; monster `--slot` refused; PC upcast with and without `upcast: true`; the warning; a PC and a monster cast of a homebrew entry; replay of a saved encounter with a homebrew entry needs no file.

## Resolution

<!-- Filled in at close. -->
