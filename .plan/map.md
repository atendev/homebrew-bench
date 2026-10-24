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
- [Engine language and runtime](closed/003-engine-language.md): Rust workspace (core library plus thin binary), shipped as prebuilt binaries via cargo-dist, mlua leads for later scripting but is undecided. See `docs/adr/0001-rust-engine-shipped-as-binaries.md`.
- [How the agent talks to the engine](closed/004-agent-engine-interface.md): one-shot `bench` CLI with JSON on stdout, a state file per encounter with a tamper check, one `bench batch` call per report, everything under `~/.bench/` with encounters optionally in `./.bench/`, install script pulls the release binary. See `docs/adr/0002-one-shot-cli-with-state-files.md`.
- [Monster schema and engine state model](closed/005-monster-schema-and-state-model.md): one step function that batch loops, no grid (engagement plus range bands), JSON stat blocks with modeled or unmodeled abilities, PCs share the schema, batch refuses unresolved unmodeled abilities, SRD import cross-checks prose. See `docs/adr/0003-step-engine-with-abstract-engagement.md`.

## Not yet specified

- How a standard party is built for monster tests: classes, gear, tactics, levels.
- The tactics the engine uses to pick creature actions in batch mode. Batch is one engine call, so the agent doesn't choose per-turn actions there. Includes the default area target count by area size and how combatants move between range bands.
- Randomness: seeds and replay. Live encounters already persist as state files, so this is about reproducing rolls.
- Override mechanics and the log format, so the log reads well to a human in both modes.
- The CR baseline that monster results are compared against.
- Which mechanics are prose-only and can't be modeled directly, and the vocabulary for the ones that can.
- Report format.
- SRD import review: how the prose-versus-structured mismatch list is produced and worked through before SRD monsters count as fixtures.
- House rule template and folder: the sections, the folder name, and how the agent finds and reads them. Includes whether house rules ever run as scripts (Lua through mlua is the leading candidate).
- Whether the SRD data is embedded in the binary or shipped as files next to it.
- How the skill files get installed into each agent (Claude Code, Codex, Cursor and others keep skills in different places), and whether `install.sh` handles it.

## Out of scope

- Non-combat play (story, social, exploration): the tool tests combats and numbers only.
- Items, spells, classes, subclasses, and the prose rules reference: separate future maps. The design should not block them.
- MCP server and long-running server modes: phase 1 is a one-shot CLI. An MCP wrapper can come later over the same commands.
- Agent-written engine code for house rules: phase 1 applies them as prose through overrides. The house rule format leaves room for it.

## Frontier

- Skill and repo name (grilling)
- Live mode turn contract (grilling)
