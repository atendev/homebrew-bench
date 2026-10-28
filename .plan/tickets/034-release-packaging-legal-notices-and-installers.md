---
id: 034
title: Release packaging, legal notices and installers
type: task
mode: AFK
status: open
blocked_by:
  - 032
  - 033
claimed: false
---

## Question

Write root `NOTICE`, `LICENSE`, and `README.md`, configure `cargo-dist` release archives in `Cargo.toml`, and build `install.sh`, `install.ps1`, and `update.sh` to install the binary, legal notices, and user-level agent skills.

## Notes

Consult `implement`, `tdd`, and `domain-modeling`.

Key decisions to implement:
- **Legal notices and root docs (`docs/adr/0013-embedded-reference-data-and-skill-packaging.md`, closed tickets 019, 021):**
  - Root `NOTICE` file containing the verbatim SRD 5.2.1 CC-BY-4.0 attribution statement with `https://www.dndbeyond.com/srd`, an explicit modification notice (stating that SRD material was reformatted, restructured, and analyzed), no other attribution to Wizards, and the `5e-database` MIT notice (matching `data/srd/README.md`).
  - Root `LICENSE` file for the repository code and `README.md` explaining what `homebrew-bench` does, how to install it (`install.sh` / `install.ps1` or from a clone), and how to invoke the skill in batch and live modes.
- **`cargo-dist` configuration (`docs/adr/0001-rust-engine-shipped-as-binaries.md`, `docs/adr/0013-embedded-reference-data-and-skill-packaging.md`, closed tickets 003, 019):**
  - Configure `[workspace.metadata.dist]` in the workspace `Cargo.toml` (and CI workflow) so every per-platform release archive bundles `bench`, `NOTICE`, `LICENSE`, and `.agents/skills/homebrew-bench/`.
- **Installers (`install.sh`, `install.ps1`, `update.sh`) (`docs/adr/0002-one-shot-cli-with-state-files.md`, `docs/adr/0013-embedded-reference-data-and-skill-packaging.md`, closed tickets 004, 010, 019):**
  - Detect platform/architecture, download the matching release archive from GitHub Releases (or build via `cargo build --release` when `--from-source` is passed or no prebuilt archive exists), and unpack `bin/bench`, `NOTICE`, `LICENSE`, and `skills/homebrew-bench/` into `~/.bench/` (`%USERPROFILE%\.bench` on Windows).
  - Create `~/.bench/config.toml` and `~/.bench/index` if missing; leave `config.toml`, `index`, `parties/`, `house-rules/`, and encounters/logs untouched on re-run or update.
  - Copy `homebrew-bench/` as plain files (not symlinks) into `~/.agents/skills/homebrew-bench/` by default, and into `~/.claude/skills/homebrew-bench/`, `~/.cline/skills/homebrew-bench/`, and `~/.kiro/skills/homebrew-bench/` whenever the corresponding binary (`claude`, `cline`, `kiro`) is on `PATH` or its `~/.<agent>` directory exists (with copy fallback on Windows for repo `.claude/skills/homebrew-bench` if symlinks are disabled).
  - Support `--no-skills`, `--agent <claude|cline|kiro|agents|all>`, `--version <tag>` (default `latest`), and `--from-source`. `update.sh` acts as a thin wrapper refreshing the install.
  - Add automated tests exercising installer flag parsing, directory layout, skill target detection, idempotent re-runs preserving `config.toml`, and `--from-source` local installation into a temporary `HOME`.

## Resolution
