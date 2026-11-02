---
id: 034
title: Release packaging, legal notices and installers
type: task
mode: AFK
status: closed
blocked_by:
  - 032
claimed: false
---

## Question

Write root `NOTICE`, `LICENSE`, and `README.md`, configure `cargo-dist` release archives in `Cargo.toml`, and build `install.sh`, `install.ps1`, and `update.sh` to install the binary, legal notices, and user-level agent skills.

## Notes

Consult `implement`, `tdd`, and `domain-modeling`.

Release notes start from `CHANGELOG.md` (Unreleased), which includes replay-compatibility notes for encounters recorded before ticket 041.

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

Built. Root `LICENSE` is MIT, from the workspace `license` field. Root `NOTICE` carries the verbatim SRD 5.2.1 statement, a modification note (reformatted, restructured, corrected, and analyzed for the baseline curve), and the 5e-database MIT text copied from `data/srd/LICENSE-5e-database.md`. Root `README.md` covers what the tool does, installing, updating, and asking an agent for batch or live mode.

`[workspace.metadata.dist]` in `Cargo.toml` (cargo-dist 0.34.0) builds `bench-<triple>.tar.gz` for macOS and Linux on x86_64 and arm64, and `.zip` for x86_64 Windows. Each archive holds `bench`, `NOTICE`, `LICENSE` and `homebrew-bench/`, with a `.sha256` beside it. dist's own installers are off. `dist generate` wrote `.github/workflows/release.yml`, which runs on version tags.

`install.sh` and `install.ps1` take `--no-skills`, `--agent <claude|cline|kiro|agents|all>` (repeatable; replaces detection), `--version <tag>` and `--from-source`. Usage errors exit 2. They verify the checksum when the `.sha256` exists. Run from a clone with no `--version`, they build the checkout. When the archive for a requested release is missing, they clone that release and build it rather than the local checkout. They write `bin/bench`, `NOTICE`, `LICENSE` and `skills/homebrew-bench/` under `~/.bench/`, create `config.toml` (matching `Config::default()`) and `index` only when missing, and copy the skill as plain files to `~/.agents/skills/` plus each detected agent. `update.sh` re-runs `install.sh`. No `update.ps1`: re-run `install.ps1`.

Deviations: no Windows fallback for the repo `.claude/skills` symlink, because the installer always copies from the canonical `.agents/skills/homebrew-bench/`.

Verification: `bench/tests/install.rs` runs `install.sh` in a sandboxed HOME and PATH against `file://` releases and a fake `cargo` (10 tests), plus an ignored test that builds with real cargo (passes). A real archive from `dist build` installed and ran. `install.ps1` was reviewed by reading only, because pwsh is not installed here, so it needs a manual Windows run before the first release. Haiku audit: round 1 raised 8 points (6 fixed, 2 rejected with sources), round 2 approved. Reports: [requirements](../reports/034-requirements.md), [implementation](../reports/034-implementation.md).
