---
id: 006
title: Spell book
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Build the loading rules from [Homebrew spell intake rules](../closed/002-homebrew-spell-intake-rules.md) ([ADR 0021](../../docs/adr/0021-homebrew-spell-entries-written-by-the-agent.md)): one spell book holds the SRD entries and any loaded homebrew entries, and every lookup goes through it.

## Notes

- Skills: `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- The global `catalog()` in `bench-core/src/spell.rs` has three call sites: `load_spell_effects` in `engine.rs`, the level lookup in `tactics.rs` (~1801) and the slot check in `stat_block.rs` (~354). Replace them with lookups that check the encounter's homebrew entries first, then the SRD.
- Homebrew entries live in `EncounterState` (serde default, skipped when empty), so replay and the tamper check (ADR 0007) don't depend on the file. Old saves load with none.
- A load function reads `--spell <file>` paths: a homebrew id equal to an SRD id is refused, and two files with one id are an error. Each message names the file and the id.
- This ticket adds `--spell` to `bench schema validate` only. It warns on a stat block cast naming an id that neither the SRD nor a loaded file supplies, and names the id. `bench start` gets the flag in [Live start with homebrew spells](007-live-start-with-homebrew-spells.md), and `bench batch` in [Batch homebrew clause resolution](011-batch-homebrew-clause-resolution.md).
- Tests: state-first lookup (a homebrew entry in state resolves; a missing id falls back to the SRD); id clash refused; duplicate refused; old saves replay byte-identical.

## Resolution

Every spell lookup goes through a `SpellBook` (`bench-core/src/spell.rs`): the encounter's homebrew entries first, then the SRD effects file. `EncounterState.homebrew_spells` holds the entries, skipped when empty, so old saves load with none and replay unchanged. A save is held to the loader rules on load: an SRD id, a repeated id or a non-homebrew entry fails to parse. `load_homebrew` reads `--spell` files and refuses a non-homebrew file, an SRD id clash and a duplicate id, each naming the file and the id. `bench schema validate` takes a repeatable `--spell`, checks cast slot levels against loaded homebrew, and warns by id on a cast no source supplies.

The three `catalog()` call sites are replaced. Engine and tactics take the book from state. The stat block's parse-time check and the batch and baseline warnings use an SRD-only book until [Live start with homebrew spells](../tickets/007-live-start-with-homebrew-spells.md) and [Batch homebrew clause resolution](../tickets/011-batch-homebrew-clause-resolution.md) pass loaded entries in.

Engine 0.1.37. The curve differs only in `engine_version`, so policy stays v5. Commit 849964f. The repo has no CI that runs on a branch push. Decisions are in `.plan/reports/006-decisions.md`.
