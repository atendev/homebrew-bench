# Ticket 042 implementation report

Ticket: `.plan/tickets/042-batch-sample-replay-command.md` (Question and Notes). Diff reviewed: `git diff` on branch `ticket-042-batch-sample-replay`, uncommitted changes. No separate requirements report exists for this ticket, so the requirements are taken from the ticket's Notes.

## What was built

- `bench-core/src/batch.rs`:
  - `BatchSpec` gains `invocation: Option<Invocation>`. `None` keeps the old behaviour for callers that have no CLI arguments.
  - New `Invocation` struct holds the arguments as passed: `monster`, `party`, `assumptions_file`, `assume`, and `assume_reason` (only when it differs from the default).
  - `replay_command` is now `pub`. With an invocation, it writes the monster argument, `--party`, `--assumptions`, each `--assume`, and `--assume-reason` as passed. Without one, it keeps the old id and party-name logic.
  - `--levels <fight level>` is added when the party lacks some level in 1 to 20.
  - New `shell_quote` wraps any value outside a plain character set in POSIX single quotes, with embedded quotes escaped as `'\''`.
  - New unit test `replay_command_quotes_the_invocation_and_names_the_level_for_partial_parties`.
- `bench/src/main.rs`:
  - `DEFAULT_ASSUME_REASON` constant, used as the clap default for `--assume-reason` and to detect a non-default reason.
  - `run_batch` builds the `Invocation` from the parsed arguments.
  - The `--fight` branch writes `replay` from `batch::replay_command`, the same function the samples use. Before, it wrote a fixed string built from the monster id.
- `bench/tests/cli.rs`:
  - `shell_words` test helper that splits a replay string the way a POSIX shell does for these cases.
  - `sample_replay_commands_rerun_homebrew_files_with_their_assumptions`: runs a homebrew file with `--assumptions`, `--assume`, and a spaced file name. Each sample's replay is parsed and rerun. The log must equal the batch log file, and the report must name the homebrew file.
  - `sample_replay_commands_keep_a_local_party_that_shadows_the_standard_name`: runs `--party standard` with a local `./.bench/parties/standard/` folder. The replay must keep `--party standard` and must rerun the local party, not the built-in one.
- `bench-core/src/baseline.rs` and `bench-core/tests/calibration.rs`: set `invocation: None` in the `BatchSpec` literals.
- `.agents/skills/homebrew-bench/references/batch-mode.md`: the replay paragraph is shortened. It now says to run the sample's `replay` string in the same working directory as the batch. The paragraph no longer explains the id and assumptions problem.
- `CHANGELOG.md`: one Unreleased entry for the replay fix.
- `.plan/tickets/042-batch-sample-replay-command.md`: `claimed` changed from `false` to `true`.

## Requirements coverage

Requirements are the ticket's Notes and Question.

- Replay must run for any input, not only SRD fixtures. Met. The monster argument as passed (a stat block path such as `brews/my ogre.json`) is written into the replay, and `shell_quote` keeps spaces and quotes intact. The test reruns a homebrew file with a space in its name.
- `--assumptions` and `--assume` must be carried, so an assumed monster is not refused on replay. Met. Both are written from `Invocation`. The test reruns a batch that needed an assumption and compares the replayed log with the batch log file.
- The original monster argument must be carried in the spec, since `BatchSpec` holds only the parsed monster. Met. `Invocation.monster` holds the argument as passed. `BatchSpec.monster` is unchanged.
- The batch must stay byte-identical for the same inputs. Met for fight results, summaries, and log files. The diff changes no fight, policy, or report code path. The only output that changes is the `replay` string. It changes in three cases: a homebrew or path monster, a call with assumptions, and a partial party, where `--levels` is now added. For a full party with a fixture monster and no assumptions, the string is the same as before.
- The `replay` string in `samples[]` and the `--fight` output must use the same logic. Met. Both call `replay_command`.
- The replay paragraph in `batch-mode.md` must be simplified. Met. The workaround text is removed. The paragraph now relies on the `replay` string.

## Deviations and extra choices

- `--levels <fight level>` is added only when the party lacks some level in 1 to 20. A full party needs no level flag, so its replay string is unchanged.
- Values are written as POSIX single-quoted words when they contain characters outside `[A-Za-z0-9-_./=:,+@%]`. Embedded single quotes use the `'\''` form.
- The replay assumes the same working directory. Relative paths stay as passed and are not made absolute.
- `--assume-reason` is written only when it differs from the default. The reason appears in the fight log header, so an omitted default still reproduces the same log.
- An explicit `--party standard` is kept in the replay, so a local `standard` folder is rerun instead of the built-in party. This is the round 1 audit fix.
- `baseline.rs` batches have no CLI invocation. Their `BatchSpec` uses `invocation: None`, and their replay uses the old id-based logic.

## Audit

- Round 1 found that an explicit `--party standard` loading a local `./.bench/parties/standard/` folder replayed against the built-in party. The replay compared `party_name` to "standard", which cannot tell the two apart. The fix carries the `--party` argument as passed in `Invocation.party`, makes `BatchSpec.invocation` an `Option`, and adds the second CLI test.
- Round 2: APPROVE.

## Verification

- `cargo test --workspace` passes.
- The rustfmt drift and the engine.rs clippy warnings were already in the repository. They were left alone.
- The sample replay test reruns each sample's replay and compares the log with the batch log file. It covers the homebrew path, assumptions, and the partial party case.

## Left for the user

- Review the `--levels` rule. It is added only for partial parties, which keeps existing full-party strings unchanged.
- Decide whether to run `cargo fmt` on the drifted files in a separate change.
- Decide whether to fix the engine.rs clippy warnings in a separate change.
- Commit the change. The ticket's Resolution section is still empty.
