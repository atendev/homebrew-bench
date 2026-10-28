# One ChaCha8 stream per encounter, seeded and replayable from a command history

Every encounter draws all of its dice from one random stream, ChaCha8 via `rand_chacha`. The algorithm is spec-frozen, so the byte stream does not depend on crate version, platform, or engine version. A given seed produces the same dice on any machine indefinitely. Rolls inside one action draw in a fixed order. The state file carries the stream as `rng: { algorithm, initial_seed, cursor }`, with the algorithm name recorded so a future RNG can be added without breaking old state files.

Seeds. Every encounter stores the seed it started from. `bench start --seed <n>` and `bench batch --seed <n>` set it; with no flag the seed comes from fresh OS entropy. A batch derives each fight's seed deterministically from the master seed and the fight index, so one sampled fight can be reproduced alone. Seeds are recorded in the encounter state, the batch report, and each fight's log header.

Reproducing a batch fight. Batch encounters have no undo and no user input, so master seed plus fight index fully determine one. `bench batch --seed X` reruns the whole batch bit-for-bit; adding `--fight i` reproduces a single fight with its full log. Materializing a batch fight into a live-style state file is a possible addition over the same machinery, not built in phase 1.

Replaying a live encounter. The state file records a command history: one canonical JSON entry per mutating command (name plus arguments), including prompt responses with user-entered die faces, override reasons, `add` and `remove`, `undo`, and `end-turn`. Read-only commands do not enter the history. `bench replay` re-executes the history from the initial snapshot and compares the re-derived digest against the recorded one, reporting a match or the index of the first diverging command.

User-entered dice draw and discard. When `pc_dice = ask`, the engine draws from the stream as it would for an engine-rolled roll, then replaces the value with the user's faces; the history records the faces. Stream position therefore depends only on the command sequence, never on the dice mode, and replay reuses the recorded faces verbatim. `bench roll` draws from the same stream. (The stream position alone does not keep the encounters identical across modes, because user faces differ from the drawn values; only the stream layout is shared.)

Undo rewinds the stream. `bench undo` restores state as of that command, including the stream cursor, so re-issuing the same command produces the same dice. UNDO lines stay in the log, so any roll-fishing is on the record.

Version caveat. A seed fixes the dice stream, not the outcomes. Rules changes between engine versions can make an old encounter diverge on replay. `bench replay` warns when the recorded engine version differs from the running one and proceeds, since where an old encounter diverges under new rules is often the interesting output.

## Considered options

- A small PRNG implemented and specified in-repo, such as xoshiro256**. Rejected because we would own the specification and the porting risk; ChaCha8's frozen spec and the `rand_chacha` crate give the same guarantee off the shelf.
- Separate streams per combatant or per roll kind. Rejected because the bookkeeping buys nothing there is a use for; one stream in a fixed draw order is fully deterministic.
- No replay machinery, with the log as the only record. Rejected because batch needs seed reruns regardless, and the command history doubles as a tamper check and debugging tool for live encounters.
- Skipping the stream draw for user-entered dice. Rejected because stream position would then depend on the dice mode, lengthening the replay contract and making cross-mode debugging harder.
- Undo without rewinding the stream, so a re-issued command gets fresh dice. Rejected because undo means the command never happened; the log's UNDO lines already make roll-fishing visible.
- Refusing to replay when the engine version differs, without a `--force`. Rejected because divergence under new rules is a debugging use case, not a mistake to prevent.
- Re-diffing the event log during replay in addition to the digest. Rejected because the log's tamper hash chain already protects it.

## Consequences

- Live state files grow an `rng` block and a command history; batch encounters write no state files, so they carry seeds only, in the report and log headers.
- A whole batch can be regenerated exactly from its master seed, and any fight in it alone by index, on any platform.
- Replay verifies a live encounter's state but cannot vouch for outcomes across engine versions; the version warning states this whenever the versions differ.
- A user who undoes and repeats a command sees the same dice. The public record shows UNDO, so this is transparent rather than hidden.
- The engine consumes stream draws even for user-entered rolls, so tooling must not treat a discarded draw as a bug.
