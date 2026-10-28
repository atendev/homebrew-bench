---
id: 033
title: Agent skill and house-rule templates
type: task
mode: AFK
status: open
blocked_by:
  - 030
  - 032
claimed: false
---

## Question

Author the agent-agnostic `homebrew-bench` skill in `.agents/skills/homebrew-bench/`, create the `.claude/skills/homebrew-bench` relative symlink, and write root `AGENTS.md`.

## Notes

Consult `writing-for-agents` (and its `SKILL-MECHANICS.md`) and `domain-modeling`. Use vocabulary from `CONTEXT.md`.

Key decisions to implement:
- **Repo layout and frontmatter contract (`docs/adr/0013-embedded-reference-data-and-skill-packaging.md`, `docs/research/agent-skill-locations.md`, closed tickets 006, 010, 019):**
  - Canonical skill directory at `.agents/skills/homebrew-bench/` containing `SKILL.md` and disclosed `references/` files where appropriate.
  - `SKILL.md` frontmatter contains only `name: homebrew-bench` and `description` so strict hosts never reject vendor-specific keys.
  - Committed relative symlink at `.claude/skills/homebrew-bench` pointing to `../../.agents/skills/homebrew-bench`.
  - Concise root `AGENTS.md` pointing to `.agents/skills/homebrew-bench/SKILL.md`.
- **Bootstrap and house rules (`docs/adr/0002-one-shot-cli-with-state-files.md`, `docs/adr/0009-house-rule-files-with-slug-tags.md`, closed tickets 004, 018, 019):**
  - First step checks for `~/.bench/bin/bench` (using `bench help --json` for exact command syntax) and runs `./install.sh --no-skills` / `./install.ps1` (or the release installer) if missing.
  - Session-start loading of house rules from `~/.bench/house-rules/` and `./.bench/house-rules/` (project layer wins on same-named slug, agent notes shadowed user-level rules; past ~10 rules, inspect title, `kind`, and `Applies when` first and disclose full file when relevant).
  - House rule authoring workflow: confirm exact wording with the user before writing, use the template frontmatter (`status: active | disabled`, `kind: gap | change`, `engine: none`) and four sections (`Rule`, `Applies when`, `How to apply`, `Notes`), and pass `--rule <slug>` on `bench override` and `bench roll`.
- **Batch mode workflow (closed tickets 001, 005, 013, 014, 015, 017):**
  - Intake: convert pasted homebrew monster text into a stat block JSON matching `bench schema monster`, record ambiguous or non-standard wording in `flags` (field path + note), and separate modeled abilities, partly modeled abilities (`agent_clauses`), inert traits, and unmodeled abilities.
  - Assumptions: walk every unmodeled ability and agent clause with the user to set a standing assumption or `"ignored"` resolution before running `bench batch`.
  - Execution and reporting: run `bench batch`, present the headline CR comparison, effective CR, difficulty bands, anomalies, wording/coverage findings, standing assumptions, active house rules, and sampled fight logs without pass/fail or unsolicited redesign advice.
- **Live mode DM workflow (`docs/adr/0004-whole-actions-with-pending-prompts.md`, `docs/adr/0006-override-paths-and-log-format.md`, closed tickets 001, 007, 011):**
  - Set up the user's party and start the encounter with `bench start` (configuring `--pc-dice ask|engine`, surprise, lighting, and starting bands).
  - Run one command per declared action, pause on `pending` prompts (routing PC reaction/dice prompts to the user and resolving monster reactions), and advance turns with `bench end-turn`.
  - Honest narration rule: state no mechanical outcome absent from `events` or `digest`; label every outside call as agent judgment and apply it via `bench override --reason [--rule <slug>]` or `bench roll --reason [--rule <slug>]`; use `--public` when the player-facing public view is active.

## Resolution
