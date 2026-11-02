---
id: 033
title: Agent skill and house-rule templates
type: task
mode: AFK
status: closed
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

Done 2026-11-02. Requirements in `.plan/reports/033-requirements.md`.

- `.agents/skills/homebrew-bench/SKILL.md` holds what every session needs: find or install `bench`, load house rules, pick a mode, read the JSON envelope, and the honest-output rules. Frontmatter is `name` and `description` only. `references/` holds `batch-mode.md`, `live-mode.md`, `house-rules.md` and a copyable `house-rule-template.md`, each read only when its branch fires.
- `.claude/skills/homebrew-bench` is a committed relative symlink to `../../.agents/skills/homebrew-bench`, and root `AGENTS.md` points to `SKILL.md`.
- `bench/tests/skill.rs` checks every `bench ...` call in the skill against `bench help --json` (commands, flags, required flags, enum values), plus the frontmatter, links, symlink, `AGENTS.md` and the template's sections.
- The skill follows the real CLI where the ticket's wording differs. Intake writes `intake_flags`, not `flags`. Lighting has no engine setting, so it is agent judgment. `--public` exists on `state` and `log` only. Batch cannot apply house rules, so the report names the ones it leaves out. Assumption values never reach the engine: `ignored` or an assumed outcome only clears the refusal, and a `partly_modeled` ability still runs its effects.
- Live mode raises dice prompts only. The skill routes reactions, opportunity attacks and legendary actions through `bench roll` and `bench override` as agent judgment.
- Two engine defects surfaced and became tickets: [Batch sample replay command for homebrew files](../tickets/042-batch-sample-replay-command.md) and [Untargeted move changes no positions](../tickets/043-untargeted-move-changes-no-positions.md). The skill works around both.
- Audit: three Haiku rounds (16 findings, all fixed), then a final pass by the implementing session that fixed four more.
