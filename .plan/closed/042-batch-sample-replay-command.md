---
id: 042
title: Batch sample replay command for homebrew files
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Make the `replay` string that `bench batch` prints for each sample (in `samples[]` and the report's sampled-log table) rerun that fight for any input, not only SRD fixtures.

## Notes

`replay_command` in `bench-core/src/batch.rs` and the `--fight` branch of `run_batch` in `bench/src/main.rs` both write `bench batch <monster id> --seed ... --fight ...`. For a homebrew file the id is not a path or fixture id, so the command fails with `not_found`. It also drops `--assumptions` and `--assume`, so a monster that needed assumptions is refused on replay. `BatchSpec` holds the parsed monster, not the argument the user passed, so the fix needs the original monster argument and the assumptions carried into the spec. Keep the batch byte-identical for the same inputs. Once fixed, simplify the replay paragraph in `.agents/skills/homebrew-bench/references/batch-mode.md`.

## Resolution

Each sample's `replay` string, and the `replay` on `bench batch --fight` output, now reruns that fight for a homebrew file and reproduces its log byte for byte.

- `BatchSpec` gains `invocation: Option<Invocation>`, which holds the `bench batch` arguments as passed: the monster argument, `--party`, `--assumptions`, each `--assume`, and `--assume-reason` when it is not the default. The reason is carried because it appears in the fight log header. `None` (the baseline and calibration callers) keeps the old rule: the monster id, plus `--party` when the party is not `standard`.
- `batch::replay_command` is public, and the `--fight` branch in `bench/src/main.rs` uses it, so both places print the same command.
- `--party` is emitted exactly when the user passed it. That covers an explicit `--party standard` that loads a local `./.bench/parties/standard/` folder (found in audit round 1).
- `--levels <fight level>` is added only when the party lacks some level in 1 to 20, since the replay otherwise checks every default level against the party.
- Values are POSIX single-quoted when they hold anything outside `[A-Za-z0-9-_./=:,+@%]`. Paths stay as passed, so the replay runs from the batch's working directory.
- Fixture batches with no `--party` or assumptions print the same replay string as before. The batch stays byte-identical for the same inputs.
- The replay paragraph in `.agents/skills/homebrew-bench/references/batch-mode.md` now says to run the `replay` string from the same working directory. The changelog says the same under Changed.
- Tests: two CLI tests run every sample's replay through a shell-word splitter and compare logs, one for a homebrew file with a space in its name and both assumption forms, one for a shadowing local `standard` party. A unit test checks quoting and the `--levels` rule. `cargo test --workspace` passes.
- Report: `.plan/reports/042-implementation.md`. Audit: haiku, two rounds, approved on round two.
