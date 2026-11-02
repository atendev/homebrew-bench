---
id: 042
title: Batch sample replay command for homebrew files
type: task
mode: AFK
status: open
blocked_by: []
claimed: false
---

## Question

Make the `replay` string that `bench batch` prints for each sample (in `samples[]` and the report's sampled-log table) rerun that fight for any input, not only SRD fixtures.

## Notes

`replay_command` in `bench-core/src/batch.rs` and the `--fight` branch of `run_batch` in `bench/src/main.rs` both write `bench batch <monster id> --seed ... --fight ...`. For a homebrew file the id is not a path or fixture id, so the command fails with `not_found`. It also drops `--assumptions` and `--assume`, so a monster that needed assumptions is refused on replay. `BatchSpec` holds the parsed monster, not the argument the user passed, so the fix needs the original monster argument and the assumptions carried into the spec. Keep the batch byte-identical for the same inputs. Once fixed, simplify the replay paragraph in `.agents/skills/homebrew-bench/references/batch-mode.md`.

## Resolution
