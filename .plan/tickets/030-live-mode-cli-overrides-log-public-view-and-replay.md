---
id: 030
title: Live mode CLI, overrides, log, public view and replay
type: task
mode: AFK
status: open
blocked_by:
  - 029
claimed: false
---

## Question

Wire the step engine into the `bench` CLI for live encounters, implementing `start`, action/move commands, `respond`, `end-turn`, `end`, `state`, `list`, `clean`, `override`, `roll`, `add`, `remove`, `undo`, `replay`, and `log [--public]`.

## Notes

Consult `implement`, `tdd`, `codebase-design`, and `domain-modeling`.

Key decisions to implement:
- **Live encounter lifecycle (`docs/adr/0002-one-shot-cli-with-state-files.md`, `docs/adr/0004-whole-actions-with-pending-prompts.md`, closed tickets 004, 007):**
  - `bench start` accepts stat block paths, sides, optional starting bands (`near` default), surprise flags, `--seed <n>`, `--pc-dice engine|ask`, and `--output home|workspace`; writes state file and log file; registers the encounter in `~/.bench/index`; prints initial JSON response (`events`, `digest`, `pending`).
  - Action and movement subcommands execute one step of the encounter; when `pending` is set, only `respond`, `undo`, `override`, and read-only commands are accepted.
  - `bench end-turn` advances the turn; `bench end --reason <text>` ends an encounter early as agent judgment; `bench state [--public]` returns the digest; `bench list` and `bench clean` manage encounters per `config.toml` (`keep_encounters`, `delete_logs`).
  - Tamper check: every log entry records a state hash, and `bench` refuses any state file whose hash does not match (`state_tampered`), or whose engine version is incompatible (except in `bench replay`).
- **Overrides, roster changes, and house-rule tags (`docs/adr/0006-override-paths-and-log-format.md`, `docs/adr/0009-house-rule-files-with-slug-tags.md`, closed tickets 011, 018):**
  - `bench override <path> [<value>|--clear] --reason <text> [--rule <slug>]`: dotted domain paths (`goblin-2.hp 14`, `mia.conditions '["prone"]'`, `mia.concentration --clear`), validated against the state schema; forbids overriding the log, encounter/combatant ids, hashes, and engine version.
  - `bench roll <expression> --reason <text> [--rule <slug>]`: draws from the encounter's ChaCha8 stream and logs `AGENT JUDGMENT`.
  - `--rule <slug>` validates `<slug>.md` against `./.bench/house-rules/` and `~/.bench/house-rules/` (rejecting unknown slugs and naming searched directories) and tags the log line with `[house rule: slug]`.
  - `bench add <statblock> --side <side> [--band near|far | --engaged-with <id>]` and `bench remove <id> --reason <text>` handle mid-fight roster changes with `ADD` and `REMOVE` log tags.
- **Undo, replay, plain-text log, and public view (`docs/adr/0006-override-paths-and-log-format.md`, `docs/adr/0007-chacha8-stream-seeds-and-replay.md`, closed tickets 011, 012):**
  - `bench undo` reverts one mutating command, restores the ChaCha8 stream cursor so re-issuing the command gets the same dice, and writes an `UNDO` line naming the reverted entry.
  - `bench replay` re-executes the recorded `command_history` (including user-entered die faces, overrides, roster changes, and undos) from the initial snapshot and seed, warns on engine version mismatch, and reports a digest match or the index of the first diverging command.
  - Plain-text log format with round/turn headers, uppercase tags (`OVERRIDE`, `UNDO`, `AGENT JUDGMENT`, `PROMPT`, `ADD`, `REMOVE`), `(player dice)` markers, and house-rule/standing-assumption tags.
  - `bench log [--public]` and `bench state --public` apply the public view filter: hide monster HP, resources, recharge state, and per-round budgets; replace monster HP with the four wounded states (`unhurt` at full, `wounded` below full, `bloodied` at half or less, `at death's door` at a quarter or less); keep override/prompt facts and reasons while dropping hidden values; show monster AC and the player side in full.

## Resolution
