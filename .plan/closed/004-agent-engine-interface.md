---
id: 004
title: How the agent talks to the engine
type: grilling
mode: HITL
status: closed
blocked_by: [001, 003]
claimed: false
---

## Question

What is the interface between the agent and the engine: CLI, library calls, MCP tools, or something else? Does one interface serve both batch and live mode?

## Notes

Live mode means the agent calls the engine many times across a conversation and reads state between calls. Batch mode means thousands of runs with no conversation.

## Resolution

Decided with the user on 2026-10-24. Recorded in `docs/adr/0002-one-shot-cli-with-state-files.md`.

- Interface: a one-shot CLI, `bench <command>`. Each call starts, does one thing, and exits. Any agent with a shell can use it, which keeps the skill agent-agnostic. MCP and a long-running server are out of scope for phase 1. An MCP wrapper over the same commands can come later without engine changes.
- One binary serves both modes. Batch mode is one `bench batch` call that plays N fights internally and returns a summary plus a few sampled full logs. The agent does not pick per-turn actions in batch. Standing assumptions go in as an override file.
- Live state: each encounter is a JSON state file that every command reads and rewrites. Each encounter records the directory it started from and a readable ID. `bench list` shows them.
- Home folder: `~/.bench/` (`%USERPROFILE%\.bench` on Windows) always exists and holds `bin/bench`, `config.toml` and `index`. The user runs the skill from any directory. Homebrew files reach the engine as path arguments, so the working directory never matters to the engine.
- Output location: `config.toml` sets `output = "home" | "workspace"`, default `home`. `workspace` puts encounters and logs in `./.bench/` under the directory the command runs from. A per-command `--output home|workspace` flag overrides the setting for one encounter. Every encounter gets a line in `~/.bench/index` with its ID and full path, so listing and resuming work from anywhere.
- Lifecycle: `config.toml` also sets `keep_encounters` (default true) and `delete_logs` (default false). `bench clean` prunes. Deleting an encounter keeps its log unless `delete_logs` is on.
- Output channels: stdout is JSON for the agent, with a `schema_version` and stable error codes. Exit codes separate ok, rules refusal, and bad input. The log is a human-readable file that only the engine writes.
- Tamper check: every log entry records a hash of the state it wrote. The engine refuses a state file that doesn't match and exits with error code `state_tampered`. Every state change goes through a logged command.
- Overrides: one `bench override` command, which rejects a call without a reason. Path syntax and how overrides read in the log go to the override and log fog item.
- Versioning: encounter files record the engine version that wrote them. The engine refuses to resume an encounter from an incompatible version and says why.
- Docs: the skill teaches the workflow. `bench help --json` prints the exact command catalog, so syntax can't drift from the installed binary.
- Install: `install.sh` and `install.ps1` in the repo detect the platform, download the latest release binary from GitHub Releases into `~/.bench/bin/`, and create `config.toml` if it's missing. Running the script again updates the binary and leaves the config alone. `update.sh` is a thin wrapper. cargo-dist still builds the binaries, and its own installers are not used. The skill's first step checks for `~/.bench/bin/bench` and runs the installer if it's missing.
- Surfaced: the live turn contract (dice, turn order, what one command covers) became its own ticket. How skill files get installed into each agent is new fog.
