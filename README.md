# homebrew-bench

Test D&D 5e homebrew monsters by running combats and reading the numbers.

`bench` is a combat engine and one-shot CLI for the 2024 rules (SRD 5.2.1). It holds the true state of every fight and does the rules math. The `homebrew-bench` agent skill teaches an AI agent (Claude Code, Codex, Cursor, Gemini CLI, Cline, Kiro and others) to drive it in two modes:

- **Batch mode.** Paste a homebrew stat block. The agent converts it to JSON, flags ambiguous wording, settles each ability the engine cannot run with you, and runs `bench batch`. That plays 100 fights per party level from 1 to 20 against a standard party of four and writes an HTML report. The report has win rate, rounds and damage per round, and compares the monster with its claimed CR on a baseline curve built from all 341 SRD monsters.
- **Live mode.** The agent DMs one combat turn by turn while you run the party. Every roll and every ruling the engine cannot make goes into a readable log, and the agent labels those rulings as agent judgment.

## Install

macOS and Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/atendev/homebrew-bench/master/install.sh | sh
```

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/atendev/homebrew-bench/master/install.ps1 | iex
```

The installer downloads the release archive for your platform and puts these in `~/.bench/` (`%USERPROFILE%\.bench` on Windows):

- `bin/bench`, the binary. Add `~/.bench/bin` to your `PATH`; the installer prints the line to use.
- `NOTICE` and `LICENSE`.
- `skills/homebrew-bench/`, the agent skill.

It then copies the skill to `~/.agents/skills/homebrew-bench/`, which most agents read, and to `~/.claude/skills/`, `~/.cline/skills/` and `~/.kiro/skills/` for each of those agents it finds on `PATH` or by its `~/.<agent>` folder.

| Option | Effect |
| --- | --- |
| `--no-skills` | Copy no skills outside `~/.bench`. |
| `--agent <name>` | Copy the skill for `claude`, `cline`, `kiro`, `agents` or `all` only, instead of detecting. Repeatable. |
| `--version <tag>` | Install this release instead of the latest. |
| `--from-source` | Build with `cargo` instead of downloading. Needs Rust from https://rustup.rs. |

To pass options through the pipe, use `curl -fsSL <url> | sh -s -- --no-skills`.

### From a clone

```sh
git clone https://github.com/atendev/homebrew-bench
cd homebrew-bench
./install.sh
```

Inside a clone, `install.sh` builds that checkout with `cargo build --release` unless you pass `--version`. When no release archive exists for your platform, the installer clones the requested release and builds it.

Agents that open the clone itself find the skill without installing it: `.agents/skills/homebrew-bench/` is the skill, `.claude/skills/homebrew-bench` links to it, and `AGENTS.md` points to it.

### Updating

Run `./update.sh`, or the install command again. Either one replaces the binary, notices and skill copies, and leaves `config.toml`, `index`, `parties/`, `house-rules/`, `encounters/` and `reports/` alone.

## Use

Ask your agent to test a monster or run a fight. Mention the skill if the agent does not pick it up by itself:

> Use homebrew-bench to test this monster against its CR: *(paste the stat block)*

> Use homebrew-bench to DM a fight between my party and four goblin warriors.

House rules are Markdown files in `~/.bench/house-rules/` or `./.bench/house-rules/`. Ask the agent to write one, and it follows `.agents/skills/homebrew-bench/references/house-rule-template.md`.

You can run `bench` without an agent. `bench help --json` lists every command and flag, `bench fixture list` lists the built-in SRD monsters, and `bench batch ogre --fights 20` tests one of them.

## Repository

- `bench-core/` is the engine library and `bench/` is the CLI.
- `data/` holds the SRD import inputs, fixtures, spell effects, standard party and baseline curve. The `bench` binary embeds the last four at compile time.
- `docs/adr/` records design decisions and `CONTEXT.md` defines the domain terms.
- `CHANGELOG.md` lists changes since the last release.

Releases are built by [cargo-dist](https://github.com/axodotdev/cargo-dist) from `.github/workflows/release.yml` when a version tag such as `v0.1.0` is pushed.

## License

The code is MIT licensed (see `LICENSE`). The game content comes from the System Reference Document 5.2.1 by Wizards of the Coast LLC under CC-BY-4.0, and the SRD data came through 5e-database under MIT. `NOTICE` has the full attribution.
