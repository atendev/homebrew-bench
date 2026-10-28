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
- The three research tickets (SRD ability survey, CR and encounter guidance, agent skill locations) ran on 2026-10-27 and are closed; findings live in `docs/research/`.

## Decisions so far

- [Destination and phase 1 scope](closed/001-destination-and-phase-1-scope.md): monsters only, batch and live both ship, one engine plus an agnostic skill, agent can override nearly anything, everything logged.
- [Rules baseline](closed/002-rules-baseline.md): 2024 rules from SRD 5.2 only, everything imported into our own format, user-added house rules kept as local files the agent applies.
- [Engine language and runtime](closed/003-engine-language.md): Rust workspace (core library plus thin binary), shipped as prebuilt binaries via cargo-dist, mlua leads for later scripting but is undecided. See `docs/adr/0001-rust-engine-shipped-as-binaries.md`.
- [How the agent talks to the engine](closed/004-agent-engine-interface.md): one-shot `bench` CLI with JSON on stdout, a state file per encounter with a tamper check, one `bench batch` call per report, everything under `~/.bench/` with encounters optionally in `./.bench/`, install script pulls the release binary. See `docs/adr/0002-one-shot-cli-with-state-files.md`.
- [Monster schema and engine state model](closed/005-monster-schema-and-state-model.md): one step function that batch loops, no grid (engagement plus range bands), JSON stat blocks with modeled or unmodeled abilities, PCs share the schema, batch refuses unresolved unmodeled abilities, SRD import cross-checks prose. See `docs/adr/0003-step-engine-with-abstract-engagement.md`.
- [Skill and repo name](closed/006-skill-and-repo-name.md): skill and repo are both `homebrew-bench`, binary stays `bench`, one skill with batch and live mode sections.
- [Live mode turn contract](closed/007-live-mode-turn-contract.md): one command resolves one whole action, pending prompts cover reactions and user-entered dice, the engine rolls by default, `bench end-turn` advances, every response carries events plus a state digest, `bench undo` reverts one command. See `docs/adr/0004-whole-actions-with-pending-prompts.md`.
- [Modeled-ability vocabulary](closed/013-modeled-ability-vocabulary.md): a closed effect list the engine runs, abilities that can be partly modeled with agent clauses, spells through an engine-owned effects file, six reaction triggers and six passive triggers, a timed modifier effect, roll modifiers with a `lighting` setting, five usage kinds, all 15 conditions, zones and spawn left unmodeled, and three classes of SRD default resolution. See `docs/adr/0005-closed-effect-vocabulary-with-agent-clauses.md`.
- [Override mechanics, log format and public view](closed/011-override-and-log-format.md): dotted override paths with `--clear`, separate `add`/`remove` commands for roster changes, one tagged plain-text log per encounter shared by both modes, and a public filter that hides monster numbers behind four wounded states. See `docs/adr/0006-override-paths-and-log-format.md`.
- [Randomness, seeds and replay](closed/012-randomness-seeds-and-replay.md): one ChaCha8 stream per encounter, `--seed` on start and batch with per-fight seeds derived from the master seed, `bench batch --seed [--fight]` reruns, live `bench replay` over a recorded command history, user-entered dice draw and discard, undo rewinds the stream. See `docs/adr/0007-chacha8-stream-seeds-and-replay.md`.
- [SRD ability survey](closed/008-srd-ability-survey.md): 1,392 ability entries across 341 stat blocks; six pattern families plus conditions and movement effects cover nearly everything, 83 entries stay unmodeled, 2 match nothing; two upstream data faults confirmed. See `docs/research/srd-ability-survey.md`.
- [CR and encounter guidance in SRD 5.2](closed/009-cr-and-encounter-guidance.md): the SRD gives only XP by CR, PB by CR and the XP budget table; no per-CR stat benchmarks, DMG "Creating a Creature" unconfirmed; CC-BY-4.0 needs verbatim attribution. See `docs/research/009-cr-and-encounter-guidance.md`.
- [Agent skill locations](closed/010-agent-skill-locations.md): `.agents/skills/` covers most agents, Claude Code needs `.claude/skills/`, Cline and Kiro need their own, `AGENTS.md` is the fallback, frontmatter stays minimal. See `docs/research/agent-skill-locations.md`.
- [SRD import review](closed/020-srd-import-review.md): four review classes with prose winning field-vs-prose and ruleset tables beating stat blocks, only mismatches block, fixtures graduate per monster toward all 341, fixes come only from `corrections.json` or the effects layer, review list regenerated and committed each run. See `docs/adr/0008-import-review-with-correction-list.md`.

## Not yet specified

- Nothing at the moment.

## Out of scope

- Non-combat play (story, social, exploration): the tool tests combats and numbers only.
- Items, spells, classes, subclasses, and the prose rules reference: separate future maps. The design should not block them.
- MCP server and long-running server modes: phase 1 is a one-shot CLI. An MCP wrapper can come later over the same commands.
- Agent-written engine code for house rules: phase 1 applies them as prose through overrides. The house rule format leaves room for it.
- Engine-played monster turns in live mode (a possible `bench auto-turn` that calls the batch tactics policy): phase 1 has the agent choose every monster action. It can be added over the same step function without changing the live contract.

## Frontier

Takeable now, in order:

- House rule template and folder (grilling)
- License and attribution notice (task)
- Standard party build (grilling)
- Packaging, SRD data and skill install (grilling)
- Seed corrections for known upstream faults (task)
- Spell effects file authorship (grilling)

Blocked:

- CR baseline (grilling), blocked by Standard party build
- Batch tactics policy (grilling), blocked by Standard party build
- Report format (prototype), blocked by Standard party build and CR baseline
