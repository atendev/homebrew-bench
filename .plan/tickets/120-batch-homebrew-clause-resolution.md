---
id: 120
title: Batch homebrew clause resolution
type: task
mode: AFK
status: open
blocked_by: [115]
claimed: false
---

## Question

Build the batch side of [Homebrew spell intake rules](../closed/002-homebrew-spell-intake-rules.md) ([ADR 0021](../../docs/adr/0021-homebrew-spell-entries-written-by-the-agent.md)): `bench batch --spell <file>` loads homebrew entries, refuses until every clause is resolved, and fingerprints each entry in the report.

## Notes

- Skills: `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- Add repeatable `--spell` to `bench batch` and to sample replay, loading through the spell book from [Spell book](115-spell-book.md).
- Pick the assumption path that names a homebrew spell's agent clauses and upcast clause (ADR 0021 leaves the path to the build), beside `abilities.<id>` in `batch.rs` (`unresolved`, ~326). Record the choice in ADR 0021.
- A homebrew entry never carries a default resolution. Batch refuses while any clause is unresolved, and the message names each path.
- The report records a fingerprint of each homebrew entry it ran, and the replay command includes the `--spell` paths.
- Tests: unresolved clause refused with its path; resolved run passes; fingerprint changes when the file changes; replay with the file reproduces the fight.

## Resolution

<!-- Filled in at close. -->
