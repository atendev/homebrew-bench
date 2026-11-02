---
name: homebrew-bench
description: Test D&D 5e homebrew monsters with the `bench` combat engine. Use when the user pastes a homebrew stat block to test or compare against its CR (batch mode), asks you to DM a combat while they run the party (live mode), or wants to add or change a house rule.
---

# Homebrew Bench

`bench` is a one-shot CLI. It holds the true state of every fight and does the rules math. You read homebrew, drive `bench`, and make the calls it cannot. Vocabulary follows `CONTEXT.md` at the repo root when you have a checkout: encounter, combatant, stat block, modeled ability, agent clause, agent judgment, override, standing assumption.

## Every session

1. **Find the binary.** Run `bench help --json`. If `bench` is not on `PATH`, try `~/.bench/bin/bench` (Windows: `%USERPROFILE%\.bench\bin\bench.exe`). If neither exists, install it:
   - In a repo checkout that has `install.sh`, run `./install.sh --no-skills` (Windows: `./install.ps1`).
   - In a checkout without an installer, run `cargo build --release -p bench`; the binary is `target/release/bench`.
   - Outside a checkout, run the installer from the latest release at https://github.com/atendev/homebrew-bench/releases.

   When the binary is not on `PATH`, put its full path wherever this skill writes `bench`. The catalog from `bench help --json` is the authority on command syntax; read it before your first command and whenever a command errors.
2. **Load house rules.** Read every `.md` file in `~/.bench/house-rules/` and `./.bench/house-rules/`. Follow [references/house-rules.md](references/house-rules.md) for layering, the ten-rule limit, and writing new rules.
3. **Pick the mode** and read its reference in full before you start it.
   - A stat block to test, compare against CR, or tune: [references/batch-mode.md](references/batch-mode.md).
   - A fight to run turn by turn with the user's party: [references/live-mode.md](references/live-mode.md).

## Reading responses

Every command prints one JSON object on stdout when it parses. The one exception is `--help`, which prints plain text. `status` is `ok` or `error`. On `ok`, the payload is under `data`. On `error`, `error.code` and `error.message` say what failed. A command that does not parse (an unknown or misplaced flag, a missing argument) prints plain text on stderr, nothing on stdout, and exits 2; check its syntax in `bench help --json`. Fix the command and retry. Leave state files alone: `bench` refuses a state file that fails its tamper check.

## Honest output

These rules hold in both modes.

- State a mechanical outcome (a hit, damage, a save, hit points, a condition) only when it appears in `events` or `digest`, or in the batch output.
- Anything else is **agent judgment**: an unmodeled ability, an agent clause, a ruling the rules leave open. Say "agent judgment" when you present it, and put it into the engine with `bench override <path> <value> --reason "<why>"` or `bench roll <dice> --reason "<why>"`. Add `--rule <slug>` when a house rule drove the call.
- `bench` writes every command and override to the log. Show the user the log when they ask what happened: `bench log`.
