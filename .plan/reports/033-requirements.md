# Ticket 033 requirements: agent skill and house-rule templates

Sources: ticket 033, ADR 0002, 0004, 0006, 0009, 0013, `docs/research/agent-skill-locations.md`, closed tickets 001, 004, 005, 006, 007, 010, 011, 013, 014, 015, 017, 018, 019, `CONTEXT.md`, `CHANGELOG.md`.
Repo state: no `.agents/`, no `.claude/skills/`, no `AGENTS.md`, no `CLAUDE.md`, no `install.sh`/`install.ps1`/`update.sh`. `.gitignore` ignores `.bench/`, `/target`, `.agent-state.md` only, so the new paths are committable. `bench` 0.1.0 builds; its catalog is checked in section F.

## A. Layout and frontmatter
1. Canonical skill at `.agents/skills/homebrew-bench/SKILL.md`, with companion files under `references/` loaded on demand (ADR 0013, ticket 019).
2. `SKILL.md` frontmatter has exactly two keys: `name: homebrew-bench` (equals the folder name; lowercase, digits, single hyphens, 1-64 chars) and `description` (1-1024 chars). No `allowed-tools`, `when_to_use`, `license`, `metadata` or other keys. `---` is the first line (ticket 010, research doc "Claude Code" and "What this means for the installer").
3. `description` names both uses: test homebrew by simulation (batch) and DM a combat live (ticket 006). It should not say "monster" in a way that blocks items/spells later (ticket 006).
4. One skill, two mode sections. Shared engine basics load first; batch and live sections load on demand, so they belong in `references/` or clearly separated sections (ticket 006).
5. `.claude/skills/homebrew-bench` is a committed relative symlink whose target is exactly `../../.agents/skills/homebrew-bench` (git mode 120000) (ADR 0013, ticket 019). `.claude/` already exists locally with an untracked `worktrees/`; commit only the symlink.
6. Root `AGENTS.md` is short and points to `.agents/skills/homebrew-bench/SKILL.md` (ticket 010, ADR 0013). Claude Code reads `AGENTS.md` only when no `CLAUDE.md` exists; none exists today, so do not add one.
7. Release packaging (ticket 034) copies `.agents/skills/homebrew-bench/` verbatim into archives and into `~/.agents/skills/`, `~/.claude/skills/`, `~/.cline/skills/`, `~/.kiro/skills/`. Skill files must therefore work as plain copies: no relative links outside the skill folder, no repo-only paths in required steps (ADR 0013).
8. Vocabulary follows `CONTEXT.md` (stat block, combatant, modeled ability, agent clause, agent judgment, override, standing assumption, house rule, rule slug, fixture, difficulty band, effective CR, thin data, casualty, pending prompt, reaction window, seed, replay, log, public view, wounded state) and avoids each term's `_Avoid_` list (ticket 033 Notes).

## B. Bootstrap
9. First step: check for `~/.bench/bin/bench` (`%USERPROFILE%\.bench\bin\bench.exe` on Windows) (ticket 004, ADR 0002).
10. If missing: inside a clone run `./install.sh --no-skills` (or `./install.ps1` on Windows); outside a clone, the release installer (ticket 019). See gap G1.
11. Then run `bench help --json` and take exact syntax from it; the skill must not duplicate flag tables that can drift (ticket 004 "Docs"). Note the envelope: `data.commands[]` and `data.engine_version`.
12. Every stdout is JSON with `schema_version`, `status`, `data` or a stable error code; the skill tells the agent to read error codes, not prose (ticket 004).

## C. House rules
13. At session start, read every `*.md` in `~/.bench/house-rules/` and `./.bench/house-rules/`. Project layer wins on a same-named slug; the agent says when a project rule shadows a user rule (ADR 0009, ticket 018).
14. Past about ten files, read only title, `kind` and `Applies when`, and open the full file when it applies (ticket 018).
15. Skip `status: disabled` rules. Choose which active rules apply to a run from the user's request; no engine switch exists (ticket 018).
16. Authoring: draft, show the exact wording to the user, write only after the user confirms. `Rule` holds the confirmed wording verbatim (ADR 0009, ticket 018, ticket 002).
17. Template: frontmatter `status: active | disabled`, `kind: gap | change`, `engine: none`; then a title; then sections `Rule`, `Applies when`, `How to apply` (the override or roll recipe), `Notes` (optional). Ship it as a file in the skill, e.g. `references/house-rule-template.md` (ADR 0009). See A1.
18. File name without `.md` is the slug. Renaming changes the slug in later logs only (ADR 0009).
19. Applying a rule: pass `--rule <slug>` on `bench override` or `bench roll`. An unknown slug is an engine error naming both folders; the skill tells the agent to fix the slug, not drop the flag (ADR 0009).
20. Where to write a new rule: user layer for rules that follow the user, project layer for one campaign; the agent asks if unclear (ticket 018).

## D. Batch mode
21. Intake: convert pasted text into a stat block that validates against `bench schema monster`; run `bench schema validate <path>` before `bench batch` (ticket 005, CLI).
22. Store the pasted text in `pasted_source` and set `source: "homebrew"` (ticket 005; field names from the schema).
23. Record each ambiguous or non-standard wording as `{path, note}` in `intake_flags` (ticket 005; the ticket says `flags`, see F).
24. Classify each ability with `kind`: `modeled` (`effects`), `partly_modeled` (`effects` + `agent_clauses`), `inert`, `unmodeled` (ticket 013). Use only the closed effect vocabulary from the schema; never invent effect types.
25. Before `bench batch`, walk every `unmodeled` ability and every agent clause with the user and record a resolution: `"ignored"` or a standing assumption (ticket 005, 013). Homebrew abilities get no default resolution.
26. Pass resolutions with `--assume abilities.<id>=<value>` (repeatable, one shared `--assume-reason`) or `--assumptions <file>` (JSON list of `{path, value, reason}`) for per-item reasons. Policy constants use `policy.<constant>` paths. No other paths are accepted (CLI, `batch.rs`).
27. A non-`ignored` ability value is a label only: the engine records it as "assumed" but runs no effect from it. If the user's assumption changes combat numbers, the agent must model it in the stat block (effects) and record the reason. See G3.
28. On `unmodeled_ability_refused`, list every offending id from the error and resolve each with the user; do not guess resolutions (ticket 031 summary, ticket 013).
29. Run `bench batch <path|fixture id>`; defaults are levels 1-20, 100 fights, 20-round cap. Echo the master seed (ticket 015, CLI).
30. Report from the JSON summary (`comparison`, `effective_cr`, `levels[].band`, `levels[].flags`, `findings`, `warnings`, `standing_assumptions`, `srd`, `samples`) and the `report.html` path, in the ticket 017 order: headline claimed vs effective CR, difficulty bands by level, anomalies and thin-data cells, wording and coverage findings (intake flags plus findings), standing assumptions, active house rules, sampled logs with their replay command (ticket 015, 017).
31. Lead with any `skipped` or missing-spell warning (ticket 013, CHANGELOG).
32. Neutral wording: no pass/fail, no unsolicited redesign advice; label bands as project calibration, not SRD numbers (ticket 015).
33. Reproduce a fight with `bench batch <monster> --seed <s> --fight <i>`; index is global across levels (CHANGELOG).
34. Custom party: `--party <name>` from `./.bench/parties/<name>/` then `~/.bench/parties/<name>/` (ticket 014).

## E. Live mode
35. Set up the user's party (their stat block files, or standard party ids from `bench party show --level <n>`) and monsters (path or fixture id; `bench fixture list`) (ticket 014, ADR 0013).
36. Start with `bench start <[side=]path[@near|far]>...`, choosing `--pc-dice engine|ask`, `--surprised <id|side>` (repeatable), optional `--group-initiative`, `--seed`, `--output`, `--id` (ticket 007, CLI).
37. With `--pc-dice ask`, `start` and `add` of a PC raise initiative prompts; the encounter stays in round 0 until `bench respond` answers (ticket 041 / CHANGELOG).
38. One command per declared action: `bench action <actor> <action> [targets...]`, `bench move <actor> <engaged|near|far> [--target] [--feet]` (ADR 0004).
39. After every command, check `pending`. While a prompt is open the engine accepts only `respond`, `undo`, `override` and read-only commands (ADR 0004).
40. Route PC prompts (reactions, dice faces) to the user and pass their answer with `bench respond <values...>` or `--decline`. The agent answers monster reaction and legendary prompts itself (ticket 007).
41. Unmodeled reactions: the engine opens no window; the agent checks them and applies them through `bench override --reason` (ADR 0004).
42. Advance only with `bench end-turn`; the engine never advances alone (ADR 0004).
43. Honest narration: state no mechanical outcome absent from `events` or `digest`. Quote event log lines where useful (ADR 0004, ticket 007).
44. Every outside call is labeled agent judgment and applied through `bench override <path> [value] --reason <text> [--rule <slug>] [--standing] [--clear]` or `bench roll <expr> --reason <text> [--rule <slug>]` (ADR 0004, 0006, 0009).
45. Roster changes use `bench add <stat_block> --side <side> [--band|--engaged-with]` and `bench remove <id> --reason`, not overrides (ADR 0006).
46. Mistakes: `bench undo` for one command; larger corrections are overrides with a reason (ADR 0004).
47. Fleeing, surrender, abandoning: `bench end --reason` (ticket 007).
48. Public view: when the user wants a player-facing view, use `bench state --public` and `bench log --public`, and narrate monster HP only as wounded states (unhurt, wounded, bloodied, at death's door) (ADR 0006). See F.
49. `bench state`, `bench log`, `bench list`, `bench replay` for inspection and resume; `--encounter <id>` targets a non-latest encounter (CLI).

## F. CLI references and catalog check (`bench help --json`, engine 0.1.0)

| Skill reference | In catalog | Note |
|---|---|---|
| `bench help --json` | yes | |
| `bench schema monster` / `schema validate <path>` | yes | |
| `bench fixture list` / `fixture show <id>` | yes | |
| `bench party show [--level]` | yes | |
| `bench baseline status` | yes | optional; `baseline generate` is a maintainer command, keep out of the skill |
| `bench batch <monster>` `--levels --fights --max-rounds --party --seed --fight --assume --assume-reason --assumptions --out --output` | yes | |
| `bench start` `--pc-dice engine\|ask` | yes | |
| `bench start` surprise | yes | flag is `--surprised`, repeatable |
| `bench start` starting bands | partly | positional suffix `path@near\|far`; no flag, no `engaged` start band |
| `bench start` lighting | **no** | no flag, no `settings.lighting` in `EncounterSettings`; `Lighting` exists only in the schema enum |
| `bench action`, `move`, `respond [--decline]`, `end-turn`, `end --reason` | yes | |
| `bench override --reason --rule --standing --clear` | yes | |
| `bench roll --reason --rule` | yes | |
| `--public` | partly | only on `bench state` and `bench log`, not on `override`/`roll`/`action`; no config setting turns it on |
| `bench add --side --band --engaged-with`, `remove --reason` | yes | |
| `bench undo`, `replay`, `list`, `clean`, `log`, `state` | yes | |
| `bench auto-turn` | no | out of scope; skill must not mention it as available |
| `./install.sh --no-skills`, `./install.ps1` | **no files** | ticket 034 |

Mismatches between ticket wording and the CLI:
- M1. "flags (field path + note)": the schema field is `intake_flags`, items `{path, note}`.
- M2. "configuring ... lighting": no lighting control exists in `bench start` or encounter state; the engine never reads lighting.
- M3. "starting bands": not a flag; `@near|far` suffix per combatant, `engaged` not allowed at start.
- M4. "use `--public`" alongside override/roll: `--public` exists only on `state` and `log`; action/override responses always carry the full digest. ADR 0006 / ticket 007 mention an optional config setting for the public view; `config.toml` has none (`output`, `keep_encounters`, `delete_logs`, `round_cap` only).
- M5. "active house rules" in batch reporting: `bench batch` takes no `--rule` and the summary has no house-rule list. House rules in batch can only enter as `--assume` paths or stat block edits, and the agent must list active rules itself.
- M6. "standing assumption" for an unmodeled ability in batch: any value other than `ignored` is recorded but has no engine effect (requirement 27).
- M7. `install.ps1` takes no stated equivalent of `--no-skills` in ticket 019/033; flag name for Windows is undecided.

## G. Dependency gaps
- G1. `install.sh`, `install.ps1`, `update.sh` and the release installer URL do not exist (ticket 034, blocked by this ticket). The bootstrap step will reference files that land later. Options: write the step against ticket 019's contract and add a dev fallback (`cargo build --release -p bench`, use `target/release/bench`) so the skill works in a clone today.
- G2. Archive layout puts the skill at `~/.bench/skills/homebrew-bench/`; the skill must not assume that path exists in a clone.
- G3. Unmodeled-ability approximations in batch need a modeling route (requirement 27). Not a blocker for the skill, but the skill text must not promise that an assumption string changes results.
- G4. Lighting (M2) and public-view config (M4) are engine gaps; the skill should omit them or label them as agent judgment, not invent flags.

## Ambiguities and open questions
- A1. Where the house-rule template ships: `references/house-rule-template.md` inside the skill (travels with installs) is the obvious choice; the ticket title implies a template file but names no path.
- A2. Should the skill create `~/.bench/house-rules/` or `./.bench/house-rules/` on first write? Installer leaves `house-rules/` untouched (ticket 019) but never says it creates it.
- A3. Batch with house rules (M5): skip, approximate through `--assume policy.*`/stat block edits, or say batch ignores prose house rules. Needs a user call.
- A4. Lighting (M2): omit from the skill, or tell the agent to apply sunlight effects as overrides with agent judgment.
- A5. How much CLI syntax to inline vs. defer to `bench help --json` (ticket 004 says defer; examples still help).

## Acceptance checklist
- [ ] `.agents/skills/homebrew-bench/SKILL.md` exists; frontmatter keys are exactly `name`, `description`; `name: homebrew-bench`.
- [ ] `description` mentions batch simulation and live DM; length within 1-1024 chars.
- [ ] `references/` holds batch, live and house-rule material loaded on demand, plus the house-rule template.
- [ ] `git ls-files -s .claude/skills/homebrew-bench` shows mode 120000 and `readlink` prints `../../.agents/skills/homebrew-bench`.
- [ ] Root `AGENTS.md` points to `.agents/skills/homebrew-bench/SKILL.md`; no `CLAUDE.md` added.
- [ ] Bootstrap: checks `~/.bench/bin/bench`, installer fallback, then `bench help --json`.
- [ ] House rules: both layers, project wins with shadow notice, ten-rule summary mode, disabled skipped, confirm-before-write, template fields and four sections, `--rule <slug>`.
- [ ] Batch: schema validate, `intake_flags`, `pasted_source`, four ability kinds, resolution walk, `--assume`/`--assumptions`, refusal handling, report order, neutral wording, `--seed --fight` replay.
- [ ] Live: `start` flags, one command per action, `pending` handling, `respond`/`--decline`, `end-turn`, honest narration, override/roll with `--reason [--rule]`, add/remove, undo, end, `state --public`/`log --public`.
- [ ] Every command and flag named in the skill exists in `bench help --json` (script or manual diff); no `--lighting`, no `auto-turn`, no `--public` on override/roll.
- [ ] No vocabulary from `CONTEXT.md` `_Avoid_` lists.
- [ ] Skill folder has no links outside itself (works as a plain copy).
- [ ] A dry run in a fresh agent session reaches a batch report and one live turn using only the skill.
