# Homebrew Bench

To test a D&D 5e homebrew monster or run a combat with the `bench` engine, read `.agents/skills/homebrew-bench/SKILL.md` and follow it.

Domain vocabulary lives in `CONTEXT.md`. Design decisions live in `docs/adr/`.

## Limit CPU use

Do not use every core. Prefix cargo commands with `nice -n 19`, for example `nice -n 19 cargo test`. `.cargo/config.toml` caps build jobs at 6 and test threads at 4; do not raise them. Pass `--threads 4` or lower to `bench baseline generate`. Slower runs are fine.
