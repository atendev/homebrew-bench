# Ticket 034 requirements: release packaging, legal notices and installers

Sources: ticket 034, `.plan/map.md`, ADR 0001, ADR 0013, `data/srd/README.md`, `.plan/reports/033-requirements.md` (sections B, F, G1, G2 and A7 cover the skill side). Not read: `CHANGELOG.md`, `CONTEXT.md`, the CLI source, `LICENSE-5e-database.md`. Items that depend on them are marked as open points.

## A. Legal notices and root docs

1. Create root `NOTICE`. It contains the SRD 5.2.1 attribution statement word for word, as in `data/srd/README.md`, including the `https://www.dndbeyond.com/srd` and `https://creativecommons.org/licenses/by/4.0/legalcode` URLs.
2. `NOTICE` contains a modification note. It says the SRD content was reformatted and restructured into JSON (wording from `data/srd/README.md`). See open point O2 on the word "analyzed".
3. `NOTICE` contains the 5e-database MIT notice: the copyright line and the MIT permission text from `LICENSE-5e-database.md`, copied verbatim. See O3.
4. `NOTICE` names no other attribution to Wizards of the Coast or its affiliates (`data/srd/README.md`, last paragraph of Licenses).
5. Create root `LICENSE` for the repository code. The license type is not chosen in any source I read. Do not pick one; see O1. Until O1 is answered this file is blocked.
6. Create root `README.md`. It must cover four things: what `homebrew-bench` does (test D&D 5e homebrew monsters by simulated combat, batch and live modes); how to install (`install.sh` or `install.ps1`, and from a clone with `--from-source`); how to invoke the skill in batch and live mode (short sample prompts only); and where the legal notices are.
7. `README.md` points to `NOTICE` for attribution. It does not repeat the Wizards attribution a second time and does not add any other Wizards or D&D attribution text.
8. `README.md` links to `SKILL.md` for the full agent instructions and does not copy them. The skill files are the single source for command syntax.
9. Write `README.md` and `NOTICE` in plain prose. Apply the `unslop` skill to both before commit, as the user's CLAUDE.md requires for text the user will read.

## B. cargo-dist configuration

10. Add `[workspace.metadata.dist]` to the workspace `Cargo.toml` (ticket 034). Before writing it, confirm the installed cargo-dist version reads this location; see O8.
11. Target triples: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`. The ticket names no list; this list is my reading of "per-platform" in ADR 0001.
12. `include` (or the cargo-dist equivalent) lists `NOTICE`, `LICENSE`, and `.agents/skills/homebrew-bench`. Each archive then contains `bench` (or `bench.exe`), `NOTICE`, `LICENSE`, and `.agents/skills/homebrew-bench/` with its `SKILL.md` and `references/` in full.
13. Archive format: `.tar.xz` for Unix targets and `.zip` for Windows, the cargo-dist default. The installers must match these names exactly.
14. The generated release workflow (`.github/workflows/release.yml`) is produced by cargo-dist and committed. CI runs `dist plan` on pull requests so a broken config fails before a tag is pushed.
15. Cargo-dist's own shell and PowerShell installers are disabled (`installers` empty or absent) so they do not compete with the hand-written `install.sh` and `install.ps1`. See O7.
16. Release notes for the first release come from the `## Unreleased` section of `CHANGELOG.md`, including the replay-compatibility notes for encounters recorded before ticket 041. I did not read the changelog, so the exact text is not verified here.

## C. install.sh layout

17. `install.sh` detects OS and architecture and maps them to one target triple from B11. An unsupported combination exits non-zero with a message that names the detected values and suggests `--from-source`.
18. Default source: download the matching archive from GitHub Releases for `--version` (default `latest`). The download URL pattern is the cargo-dist default for the repository. See O11.
19. `--from-source` (or no matching archive for the chosen version) runs `cargo build --release -p bench` in the checkout and installs `target/release/bench`. See O6 for the clone case.
20. Installed layout under `~/.bench/`:
    - `bin/bench` (mode 0755)
    - `NOTICE`
    - `LICENSE`
    - `skills/homebrew-bench/` (copied from the archive, or from the checkout's `.agents/skills/homebrew-bench/` for `--from-source`)
    ADR 0013 and ticket 033 G2 both give this layout.
21. The installer overwrites only `bin/bench`, `NOTICE`, `LICENSE`, and `skills/homebrew-bench/`. It writes nothing else under `~/.bench/`.
22. The skill directory is replaced as a whole on each run, so files removed upstream do not linger. Replacing it does not touch any other path under `~/.bench/`.
23. Create `~/.bench/config.toml` only if it is missing. Its content is the defaults `bench` reads (`output`, `keep_encounters`, `delete_logs`, `round_cap`, per ticket 033 M4). See O13 on how those defaults are sourced.
24. Create `~/.bench/index` (empty) only if it is missing.
25. The installer never edits shell profiles or `PATH`. It prints the full path to `bench` on success. The skill bootstrap (033 B9) calls the full path, so no PATH change is needed.
26. A missing `~/.bench/` is created with mode 0755 (directories) and the parent `HOME` is never changed.

## D. install.sh flags

27. `--no-skills` installs the binary and notices but writes no skill copies in any agent folder. It still refreshes `skills/homebrew-bench/` under `~/.bench/`, because that copy is part of the release bundle.
28. `--agent <claude|cline|kiro|agents|all>` selects agent targets (section E). It may repeat. It is an error to combine it with `--no-skills`.
29. `--version <tag>` defaults to `latest`. A tag that has no release exits non-zero before any file is written.
30. `--from-source` as in requirement 19.
31. `--help` prints usage and exits 0. An unknown flag exits 2 with the usage text and writes nothing.
32. Exit codes: 0 success, 1 runtime failure (download, build, copy), 2 usage error. Tests check these.

## E. Skill target detection

33. Default (no `--agent`): always write `~/.agents/skills/homebrew-bench/`. Write `~/.claude/skills/homebrew-bench/`, `~/.cline/skills/homebrew-bench/`, and `~/.kiro/skills/homebrew-bench/` only when the agent's binary (`claude`, `cline`, `kiro`) is on `PATH` or its `~/.<agent>` directory exists.
34. `--agent all` writes all four targets regardless of detection.
35. `--agent <name>` (one or more) writes exactly the named targets. It does not add `~/.agents/` unless `agents` is named. See O4.
36. Skill copies are plain files and directories, never symlinks (ADR 0013, ticket 034). Use copy with `-R` semantics and check that no symlink is created.
37. Each target is replaced as a whole directory (same rule as 22).
38. Detection reports what it did: one line per target written, one line per target skipped with the reason ("not detected"). A test reads this output.
39. There is no repo-level `.claude/` copy and no installer write inside the checkout for skills. The committed symlink from ticket 033 is the only repo-level skill link. See O5.

## F. update.sh

40. `update.sh` is a thin wrapper: it runs `install.sh` with the same arguments (default `--version latest`) and prints nothing extra beyond the installer's own output.
41. Re-running `install.sh` is the update path on Windows too. `update.sh` adds no state of its own.

## G. install.ps1

42. `install.ps1` mirrors sections C to E for Windows: install root `%USERPROFILE%\.bench`, binary at `bin\bench.exe`, archive `bench-x86_64-pc-windows-msvc.zip`, same preserved paths, same skill targets under `%USERPROFILE%\.agents\skills\`, `%USERPROFILE%\.claude\skills\`, etc.
43. Flag names match `install.sh` (`-NoSkills` or `--no-skills`; see O10 on the spelling). Exit codes match requirement 32.
44. Copies are never symlinks, so requirement 36's copy rule applies without the symlink fallback in the ticket (see O5).

## H. Preservation and idempotence (all platforms)

45. Paths that re-runs and updates must leave untouched, byte for byte: `~/.bench/config.toml`, `~/.bench/index`, `~/.bench/parties/`, `~/.bench/house-rules/`, and any encounter or log files under `~/.bench/`. A test writes a sentinel into each and checks it survives a second install.
46. A second run with the same version produces the same tree, apart from the refreshed binary and skill copies.
47. Installing into a `HOME` that already has `~/.bench/` from an older run keeps every file in requirement 45 and replaces only requirement 21's files.

## I. Tests

48. Tests run against a temporary `HOME` (`mktemp -d` on Unix, `$env:TEMP` subfolder on Windows) and never touch the real `HOME`. The test asserts this by setting `HOME` (and `USERPROFILE` on Windows) to the temp dir before running the script.
49. Required test cases (ticket 034 list, expanded):
    - flag parsing: each flag, each bad value, unknown flag exits 2, `--agent` with `--no-skills` exits 2;
    - directory layout after `--from-source` into a temp `HOME`: `bin/bench` executable, `NOTICE`, `LICENSE`, `skills/homebrew-bench/SKILL.md` present;
    - skill target detection with fake `claude` on `PATH` (a stub script) and a fake `~/.kiro`, checking exactly the expected targets are written;
    - `--agent all` writes four targets; `--no-skills` writes none outside `~/.bench/`;
    - idempotent re-run preserves the sentinel files from requirement 45;
    - no symlinks are created under `HOME` by any run.
50. Shell tests run in CI on Linux and macOS. PowerShell tests run in CI on Windows, or the ticket records them as manual if no Windows job exists. See O10.
51. `--from-source` tests must not download anything. Provide a test-only override (an environment variable, for example `BENCH_INSTALL_ARCHIVE_DIR`) so the download path can be tested against a local archive built with `dist build`. The override is documented in the script header and is not in `--help`.
52. Requirement 19 needs a Rust toolchain in the test job; that is acceptable because the workspace already builds in CI.

## J. Acceptance checklist

- [ ] `NOTICE`, `LICENSE`, `README.md` exist at repo root; `NOTICE` text matches `data/srd/README.md` for the statement and modification note.
- [ ] `dist plan` passes; archive contents listed in requirement 12 are present in a local `dist build` output.
- [ ] `install.sh --from-source` into a temp `HOME` gives the layout in requirement 20.
- [ ] Sentinel files in requirement 45 survive a re-run and an update through `update.sh`.
- [ ] No symlinks created by any installer run.
- [ ] `install.ps1` tests pass on Windows, or the ticket records them as manual.
- [ ] `README.md` sample prompts match the skill's batch and live sections.

## Open points

O1. LICENSE for the repository code. No source picks a license, and the ticket says only "LICENSE file for the repository code". Recommend asking the user before writing the file. The README install section also needs the license name.

O2. Modification wording. The ticket asks for "reformatted, restructured, and analyzed". `data/srd/README.md` says "reformatted and restructuring into JSON", and ticket 021 closed on that text. "Analyzed" describes project work on the upstream content, not a change to the SRD text. Recommend using the `data/srd/README.md` wording unchanged, so `NOTICE` and the data README cannot disagree.

O3. The 5e-database MIT notice. `data/srd/README.md` refers to `LICENSE-5e-database.md` but I did not confirm the file exists or read its copyright line. Recommend copying that file's notice text into `NOTICE` exactly, and failing the build if the file is missing.

O4. `--agent` semantics. ADR 0013 says `--agent <name|all>`; the ticket says `<claude|cline|kiro|agents|all>`. Recommend: an explicit `--agent` replaces auto-detection and writes only the named targets, so `--agent claude` does not write `~/.agents/`. Users who want both name both.

O5. Windows symlink fallback. The ticket says "copy fallback on Windows for repo `.claude/skills/homebrew-bench` if symlinks are disabled", which asks the installer to write inside the repo. ADR 0013 and ticket 033 put that symlink in git, and the installer writes only user-level copies. Recommend dropping the fallback. If Windows checkouts lose the symlink, the user gets the `.agents` copy anyway.

O6. Install from a clone. Ticket 033 G1 says the skill's bootstrap runs `./install.sh --no-skills` inside a clone before any release exists. Recommend: when the script runs inside a checkout and no `--version` is given, build from source; a release is used only when the user passes `--version` or runs from outside a checkout. Confirm this with the user, since it changes what `install.sh` downloads by default.

O7. Generated cargo-dist installers. cargo-dist can generate its own `install.sh` and `install.ps1`, which would clash with the hand-written ones. Recommend disabling the generated installers (requirement 15).

O8. cargo-dist config file. Recent cargo-dist releases read `dist-workspace.toml` rather than `[workspace.metadata.dist]`. The ticket says `Cargo.toml`. Recommend checking the installed cargo-dist version first. If it supports only the newer file, follow the tool and note the deviation in the ticket.

O9. `update.sh` on Windows. The ticket names only `update.sh`. Recommend no `update.ps1`; section F's re-run path covers Windows.

O10. Test runner and flag spelling. The repo has no shell or PowerShell test harness, and the ticket says to consult `tdd` without naming one. Recommend plain bash tests under `tests/installer/` for Unix and a PowerShell script for Windows. PowerShell flags can be `-NoSkills` or `--no-skills`; recommend `--no-skills` on both so the docs say one thing.

O11. Repository owner and name for download URLs. Neither source gives the GitHub owner. Confirm from the git remote before writing the download URL.

O12. Checksums. cargo-dist writes `.sha256` files next to each archive. The ticket does not ask for verification. Recommend verifying when the checksum file is present and failing on mismatch, since the installer is a download-and-run path. Left out of the numbered requirements because the ticket does not ask for it.

O13. Default `config.toml` content. Requirement 23 needs the defaults `bench` reads, and I did not read the config code. Recommend the installer write the file from a template kept next to the installer, or a `bench` subcommand if one exists, so the defaults have one source.
