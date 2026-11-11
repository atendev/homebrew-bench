---
id: 115
title: Spell book
type: task
mode: AFK
status: open
blocked_by: [114]
claimed: false
---

## Question

Build the loading rules from [Homebrew spell intake rules](../closed/002-homebrew-spell-intake-rules.md) ([ADR 0021](../../docs/adr/0021-homebrew-spell-entries-written-by-the-agent.md)): one spell book holds the SRD entries and any loaded homebrew entries, and every lookup goes through it.

## Notes

- Skills: `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- The global `catalog()` in `bench-core/src/spell.rs` has three call sites: `load_spell_effects` in `engine.rs`, the level lookup in `tactics.rs` (~1801) and the slot check in `stat_block.rs` (~354). Replace them with lookups that check the encounter's homebrew entries first, then the SRD.
- Homebrew entries live in `EncounterState` (serde default, skipped when empty), so replay and the tamper check (ADR 0007) don't depend on the file. Old saves load with none.
- A load function reads `--spell <file>` paths: a homebrew id equal to an SRD id is refused, and two files with one id are an error. Each message names the file and the id.
- This ticket adds `--spell` to `bench schema validate` only. It warns on a stat block cast naming an id that neither the SRD nor a loaded file supplies, and names the id. `bench start` gets the flag in [Live start with homebrew spells](116-live-start-with-homebrew-spells.md), and `bench batch` in [Batch homebrew clause resolution](120-batch-homebrew-clause-resolution.md).
- Tests: state-first lookup (a homebrew entry in state resolves; a missing id falls back to the SRD); id clash refused; duplicate refused; old saves replay byte-identical.

## Resolution

<!-- Filled in at close. -->
