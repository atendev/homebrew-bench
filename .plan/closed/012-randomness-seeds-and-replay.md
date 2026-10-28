---
id: 012
title: Randomness, seeds and replay
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

How does the engine produce and reproduce rolls? Is there a seed per encounter and per batch? Does a batch report record its seed so a sampled fight can be replayed? How does replay treat user-entered dice, `bench roll`, overrides and undo? Where does the random state live in the state file, and which RNG algorithm is fixed so results match across platforms and engine versions?

## Notes

Live encounters already persist as state files, so this is about reproducing rolls, not resuming. The state-model ticket listed randomness state in the encounter file and left its shape here. The live-mode ticket added user-entered dice and `bench roll`. Consult `grilling` and `domain-modeling`.

## Resolution

Decided with the user on 2026-10-27. Recorded in `docs/adr/0007-chacha8-stream-seeds-and-replay.md`. New terms (seed, command history, replay) are in `CONTEXT.md`.

- Every encounter draws all dice from one ChaCha8 stream in a fixed order, recorded as `rng: { algorithm, initial_seed, cursor }`. A given seed produces the same dice on any platform and engine version.
- `bench start --seed` and `bench batch --seed` set the seed; the default is fresh OS entropy. A batch derives each fight's seed from the master seed and fight index. Seeds are recorded in the encounter state, the batch report, and every fight's log header.
- `bench batch --seed X` reruns a whole batch bit-for-bit, and `--fight i` reproduces one sampled fight with its full log. Materializing a batch fight into a live-style state file is a possible addition over the same machinery, not phase 1.
- Live encounters record a command history, one canonical JSON entry per mutating command, including prompt answers with user-entered faces, override reasons, and undo. `bench replay` re-executes the history from the initial snapshot and diffs the digest, reporting a match or the first diverging command index. The event log is not re-diffed; its tamper hash chain already covers it.
- User-entered dice draw and discard, so stream position depends only on the command sequence, never on the dice mode. Replay reuses the recorded faces. `bench roll` draws from the same stream.
- Undo restores state including the stream cursor, so a re-issued command gets the same dice. The UNDO line makes any roll-fishing visible.
- A seed fixes the dice stream, not outcomes across engine versions. `bench replay` warns on a version mismatch and proceeds, since the divergence point is the useful output.

**Surfaced.** Nothing new. Not yet specified stays empty; this ticket unblocked nothing.
