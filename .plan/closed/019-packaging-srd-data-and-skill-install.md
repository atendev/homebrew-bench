---
id: 019
title: Packaging, SRD data and skill install
type: grilling
mode: HITL
status: closed
blocked_by: [010]
claimed: false
---

## Question

Is the SRD data embedded in the binary or shipped as files next to it? How do the skill files get installed into each agent, and does `install.sh` handle it? What does the release archive contain?

## Notes

The interface ticket fixed `install.sh` and `install.ps1`, which download the release binary into `~/.bench/bin/`. Use the agent skill locations findings (`docs/research/agent-skill-locations.md`): `.agents/skills/` covers most agents, Claude Code needs `.claude/skills/`, Cline and Kiro need their own paths, and detection is binary-plus-config-dir inference. Open caveat from the findings: Antigravity CLI (which replaced Gemini CLI for unpaid tiers in June 2026) is unresearched. The release archive must carry the SRD attribution notice as a `NOTICE` or `LICENSE-SRD` file: the verbatim SRD 5.2.1 statement plus a modification note, per the closed License and attribution notice ticket; a linked notice is acceptable. Consult `grilling` and `domain-modeling`.

## Resolution

Resolved 2026-10-28 by grilling. Recorded in `docs/adr/0013-embedded-reference-data-and-skill-packaging.md`.

- **Compile-time embedded reference data.** `data/fixtures/` (graduated SRD stat blocks), `data/spells/` (engine-owned spell effects), `data/party/` (progression tables and 80 generated standard-party stat blocks), and `data/baseline/` (empirical curve and input fingerprints) are committed in the repo and embedded into `bench` at compile time (`include_str!` / `include_bytes!`). `data/srd/2024/` and `data/srd/corrections.json` remain in the repo as import inputs only. The binary has no runtime data-file dependencies and its embedded data cannot drift from its baseline fingerprint.
- **Referencing and inspecting embedded data.** `bench start` and `bench batch` accept either a file path on disk or a bare fixture slug resolved against embedded fixtures. Read-only inspection commands (`bench fixture list`, `bench fixture show <id>`, and `bench party show [--level <1-20>]`) dump the embedded JSON to stdout so the agent and user never need loose files under `~/.bench/data/`.
- **Both entry paths supported.** Working inside a cloned repo works out of the box via project-level skill files (and the skill's first step runs `./install.sh --no-skills` / `./install.ps1` if `~/.bench/bin/bench` is missing). Running `install.sh` or `install.ps1` directly (or piped from a release URL) installs both the binary and user-level skills without requiring a repo clone.
- **Repo skill layout and frontmatter contract.** The canonical skill lives at `.agents/skills/homebrew-bench/` (`SKILL.md` plus any companion files in `references/`). `.claude/skills/homebrew-bench` is a committed relative symlink (`../../.agents/skills/homebrew-bench`, with a copy fallback on Windows in `install.ps1` if symlinks are disabled), and a short root `AGENTS.md` points to `.agents/skills/homebrew-bench/SKILL.md`. `SKILL.md` frontmatter uses only `name: homebrew-bench` and `description` so strict hosts never reject vendor-specific keys.
- **Release archive and `NOTICE`.** Every per-platform release archive built by `cargo-dist` (`[workspace.metadata.dist]` `include`) bundles `bench`, `NOTICE` (carrying the verbatim SRD 5.2.1 CC-BY-4.0 statement, modification note, and 5e-database MIT notice), `LICENSE`, and `.agents/skills/homebrew-bench/`. `install.sh` and `install.ps1` unpack `bin/bench`, `NOTICE`, `LICENSE`, and `skills/homebrew-bench/` into `~/.bench/` so the legal notice stays with the installed binary.
- **User-level skill install and detection.** By default, `install.sh` and `install.ps1` copy `homebrew-bench/` into `~/.agents/skills/homebrew-bench/` (covering Codex, Cursor, Gemini CLI, Antigravity CLI, GitHub Copilot, Windsurf/Devin, opencode, Amp, Roo Code, Goose, Junie, and Zed; runtime verification confirmed Antigravity mounts `.agents/skills/` and `~/.agents/skills/`) and into `~/.claude/skills/`, `~/.cline/skills/`, and `~/.kiro/skills/` whenever the corresponding binary (`claude`, `cline`, `kiro`) is on `PATH` or its `~/.<agent>` directory exists. Plain copies (not symlinks) are written into user-level skill folders so agent sandboxes never reject cross-directory symlinks.
- **Installer flags, updates, and source fallback.** `--no-skills` skips user-level skill copies, `--agent <claude|cline|kiro|agents|all>` forces targets, `--version <tag>` (default `latest`) pins or rolls back to a specific release to match an older encounter's engine version, and `--from-source` (or a missing platform archive) falls back to `cargo build --release`. Re-running `install.sh` or `update.sh` refreshes `bin/bench`, `NOTICE`, `LICENSE`, and installed skill copies while leaving `config.toml`, `index`, `parties/`, `house-rules/`, and encounters/logs untouched.

Surfaced: whether a dedicated ticket is needed for authoring the `homebrew-bench` `SKILL.md` prose once Report format closes.
