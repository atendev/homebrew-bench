# Map: homebrew-bench

## Destination

A repo someone downloads that tests D&D 5e homebrew without running a game. They give a skill a homebrew monster, item, spell, class, or subclass as prose or a stat block. The agent converts it to a structured form and flags ambiguous wording. A combat engine in code runs the fights and does the math. The agent then writes a report with the numbers, a short readout, and the wording problems. Anything the engine can't model is labeled as agent judgment. Phase 1 is done when a homebrew monster goes from pasted text to a report covering win rate, rounds, and damage per round against a standard party at several levels, compared with DMG CR guidelines.

## Decisions so far

- 001 Architecture and SRD import. A skill sits on top of a harness made of SRD data, a homebrew schema, a combat engine in code, per-type scenarios, and a report format. SRD 5.1 and 5.2 JSON are pinned in `data/srd/`. [closed/001-architecture-and-srd-import.md]
- 002 Baseline ruleset. The engine uses 2024 rules (SRD 5.2), and intake assumes homebrew is written for 2024. [closed/002-baseline-ruleset.md]
- 005 Prose ability survey. Condition riders on attacks, area sizes, conditional advantage, Legendary Resistance, and legendary actions are the main prose-only mechanics. Legendary actions per round is missing from the data. [closed/005-survey-prose-abilities.md]
- 003 Engine language. TypeScript on Bun. [closed/003-engine-language.md]

## Not yet specified

- How a "standard party" is built for monster tests: which classes, gear, and tactics at each level.
- How the agent and the engine talk: CLI, library calls, or MCP tools.
- The 2024 CR baseline for monster comparisons: a published table, or one derived from SRD 5.2 monsters.
- Report format details beyond "numbers, readout, intake problems".
- Phases 2 to 4: items and spells as modifiers, classes and subclasses, the prose rules reference.

## Frontier

- 004 Monster schema and engine state model (grilling)
- 007 Skill and repo name (grilling)
