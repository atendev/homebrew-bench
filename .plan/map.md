# Map: homebrew-bench

## Destination

A repo someone downloads to test D&D 5e homebrew monsters by running combats and reading the numbers, aimed at DMs first and homebrew designers second. One engine holds combat state and does the rules math. An agent-agnostic skill teaches any LLM how to use it, in two modes:

- Batch mode: the agent converts pasted monster text to a structured form, flags ambiguous wording, runs many fights, and writes a report with win rate, rounds, and damage per round across party levels, compared with CR guidance.
- Live mode: the agent DMs one full combat encounter turn by turn through the engine while the user controls the party.

Both modes ship in phase 1. Everything the engine can't model is labeled agent judgment. Every engine action goes into a human-readable log.

## Notes

- Domain: D&D 5e homebrew testing. Consult the `grilling` and `domain-modeling` skills in every session. Vocabulary lives in `CONTEXT.md`.
- Existing material: `data/srd/` holds SRD JSON from an earlier pass and `scripts/` holds import tooling. The 2024 data stays as import source. The 2014 data is slated for removal (see Rules baseline).
- The map was reset on 2026-10-24. Earlier decisions were discarded on purpose and live only in git history (commit 9120650).

## Decisions so far

- [Destination and phase 1 scope](closed/001-destination-and-phase-1-scope.md): monsters only, batch and live both ship, one engine plus an agnostic skill, agent can override nearly anything, everything logged.
- [Rules baseline](closed/002-rules-baseline.md): 2024 rules from SRD 5.2 only, everything imported into our own format, user-added house rules kept as local files the agent applies.

## Not yet specified

- How a standard party is built for monster tests: classes, gear, tactics, levels.
- Who chooses each creature's actions in batch mode versus live mode, and how the engine takes those choices as input.
- The contract between the agent-as-DM and the engine in live mode: who rolls dice, how the agent stays honest to the engine's state.
- Randomness: seeds, replay, and how a live fight is saved and resumed.
- Override mechanics and the log format, so the log reads well to a human in both modes.
- The CR baseline that monster results are compared against.
- Which mechanics are prose-only and can't be modeled directly, and the vocabulary for the ones that can.
- Report format.
- House rule template and folder: the sections, the folder name, and how the agent finds and reads them.

## Out of scope

- Non-combat play (story, social, exploration): the tool tests combats and numbers only.
- Items, spells, classes, subclasses, and the prose rules reference: separate future maps. The design should not block them.
- Agent-written engine code for house rules: phase 1 applies them as prose through overrides. The house rule format leaves room for it.

## Frontier

- Engine language and runtime (grilling)
- Skill and repo name (grilling)
