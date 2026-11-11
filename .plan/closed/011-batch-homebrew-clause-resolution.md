---
id: 011
title: Batch homebrew clause resolution
type: task
mode: AFK
status: closed
blocked_by: [006]
claimed: false
---

## Question

Build the batch side of [Homebrew spell intake rules](../closed/002-homebrew-spell-intake-rules.md) ([ADR 0021](../../docs/adr/0021-homebrew-spell-entries-written-by-the-agent.md)): `bench batch --spell <file>` loads homebrew entries, refuses until every clause is resolved, and fingerprints each entry in the report.

## Notes

- Skills: `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- Add repeatable `--spell` to `bench batch` and to sample replay, loading through the spell book from [Spell book](006-spell-book.md).
- Pick the assumption path that names a homebrew spell's agent clauses and upcast clause (ADR 0021 leaves the path to the build), beside `abilities.<id>` in `batch.rs` (`unresolved`, ~326). Record the choice in ADR 0021.
- A homebrew entry never carries a default resolution. Batch refuses while any clause is unresolved, and the message names each path.
- The report records a fingerprint of each homebrew entry it ran, and the replay command includes the `--spell` paths.
- Tests: unresolved clause refused with its path; resolved run passes; fingerprint changes when the file changes; replay with the file reproduces the fight.

## Resolution

`bench batch --spell <file>` (repeatable) loads entries through the spell book and copies them into every fight, so the start-time checks apply. A homebrew entry has no default: batch refuses until every `agent_clauses` item and the `upcast_clause` has a standing assumption at `spells.<id>.agent_clauses.<n>` (from 0, in file order) or `spells.<id>.upcast_clause`. The refusal names each path, in the message and in `details.spell_clauses`. A `spells.` path with a blank value, the wrong shape or no matching loaded clause is `invalid_argument`. The path is recorded in ADR 0021.

Each resolved clause is a report finding. The summary gains `homebrew_spells` with each entry's fingerprint (omitted without `--spell`), the assumptions table and each log header name the entries, and a sample's replay command repeats the `--spell` paths, so replay reproduces the fight.

Engine 0.1.41. The curve differs only in `engine_version`, so policy stays v5. Commit 2ab9111 (amended to the SHA at merge). The repo's only workflow is `release.yml`, which does not run on a branch push, so no CI ran. Decisions are in `.plan/reports/011-decisions.md`. Left open: a blank value on `abilities.<id>` still passes, and replay does not compare a spell file's fingerprint with the report's.
