# Agent skill and instruction file locations

Research for ticket 010. Gathered 2026-10-24 from each agent's own documentation. This feeds the Packaging work: the project ships one agent-agnostic skill named `homebrew-bench`, and `install.sh` / `install.ps1` may copy it into each agent's location.

## Method and caveats

- Sources are official vendor docs only. Every claim below carries a URL. A few vendors moved their docs, so the fetched URL differs from the one I started with. Each is noted where it matters.
- Pages were read through a fetch tool that summarizes pages with a small model. Paths and field names were taken from those summaries, not from the raw HTML. Before the installer hard-codes a path, spot-check it against the linked page.
- Nothing here was tested by running the agents. "Detection" columns are design suggestions built from documented binary names, install commands and config paths. No vendor documents a "detect this agent" procedure, so detection is my inference throughout and is labelled that way.
- Docs for this space change fast. Several agents added or renamed skill support within the last year. Treat the table as a snapshot for 2026-10-24.

## The standard that most agents follow

The Agent Skills specification defines a skill as a directory with a required `SKILL.md`, optional `scripts/`, `references/` and `assets/`, and YAML frontmatter. `name` (1 to 64 chars, lowercase letters, digits, single hyphens, must match the parent directory name) and `description` (1 to 1024 chars) are required. `license`, `compatibility` (max 500 chars), `metadata` (string to string map) and `allowed-tools` (experimental) are optional. Source: https://agentskills.io/specification

The spec does not define install locations. The locations come from each agent. The pattern in the docs is that `.agents/skills/` (project) and `~/.agents/skills/` (user) are now a shared cross-agent location, while Claude Code and Cline still centre on their own directories.

## Summary table

"Skills" means the `SKILL.md` folder convention. "Instr." means the single-file or rules-file fallback. Paths are relative to the repo root for project level and to the home directory for user level.

| Agent | Project skills | User skills | Reads `SKILL.md`? | Project instructions | User instructions | Detection idea (inferred) |
|---|---|---|---|---|---|---|
| Claude Code | `.claude/skills/<n>/SKILL.md` | `~/.claude/skills/<n>/SKILL.md` | Yes | `CLAUDE.md`, `.claude/CLAUDE.md`, `CLAUDE.local.md`, `AGENTS.md` (only when no CLAUDE.md) | `~/.claude/CLAUDE.md` | `command -v claude`, or `~/.claude` exists |
| OpenAI Codex CLI | `.agents/skills/` from cwd up to repo root | `~/.agents/skills/` (admin: `/etc/codex/skills`) | Yes | `AGENTS.md`, `AGENTS.override.md` | `~/.codex/AGENTS.md` (or `$CODEX_HOME`) | `command -v codex`, or `~/.codex` exists |
| Cursor | `.agents/skills/`, `.cursor/skills/` (also reads `.claude/skills/`, `.codex/skills/`) | `~/.agents/skills/`, `~/.cursor/skills/` (also `~/.claude/skills/`, `~/.codex/skills/`) | Yes | `.cursor/rules/*.mdc`, `AGENTS.md` | Rules set in the Cursor settings UI, no file path documented | `~/.cursor` exists, or `command -v agent` (CLI) |
| Gemini CLI | `.gemini/skills/` or `.agents/skills/` | `~/.gemini/skills/` or `~/.agents/skills/` | Yes | `GEMINI.md` (name configurable via `context.fileName`) | `~/.gemini/GEMINI.md` | `command -v gemini`, or `~/.gemini` exists |
| GitHub Copilot | `.github/skills/`, `.claude/skills/`, `.agents/skills/` | `~/.copilot/skills/`, `~/.agents/skills/` (VS Code also reads `~/.claude/skills/`) | Yes | `.github/copilot-instructions.md`, `.github/instructions/**/*.instructions.md`, `AGENTS.md`, `CLAUDE.md`, `GEMINI.md` | Not confirmed | `command -v copilot` (CLI), or `~/.copilot` exists; VS Code extension needs a separate check |
| Windsurf (docs now under Devin) | `.devin/skills/` (legacy `.windsurf/skills/`), also `.agents/skills/` | `~/.codeium/windsurf/skills/`, `~/.config/devin/skills/`, also `~/.agents/skills/` | Yes | `.devin/rules/*.md` (fallback `.windsurf/rules/`, legacy `.windsurfrules`), `AGENTS.md` | `~/.codeium/windsurf/memories/global_rules.md` | `~/.codeium/windsurf` exists |
| Cline | `.cline/skills/`, `.clinerules/skills/`, `.claude/skills/` | `~/.cline/skills/` only | Yes | `.clinerules/` or `.cline/rules/`, `AGENTS.md`, also detects `.cursorrules`, `.windsurfrules` | `~/Documents/Cline/Rules`, `~/.cline/rules`, `~/.agents/AGENTS.md` | `command -v cline` (CLI), or `~/.cline` exists |
| opencode | `.opencode/skills/`, `.claude/skills/`, `.agents/skills/` | `~/.config/opencode/skills/`, `~/.claude/skills/`, `~/.agents/skills/` | Yes | `AGENTS.md`, fallback `CLAUDE.md` | `~/.config/opencode/AGENTS.md`, fallback `~/.claude/CLAUDE.md` | `command -v opencode`, or `~/.config/opencode` exists |
| Amp | `.agents/skills/`, `.claude/skills/` | `~/.config/agents/skills/`, `~/.agents/skills/`, `~/.config/amp/skills/`, `~/.claude/skills/` | Yes | `AGENTS.md` (fallback `AGENT.md`, `CLAUDE.md`) | `~/.config/amp/AGENTS.md`, `~/.config/AGENTS.md` | `command -v amp`, or `~/.config/amp` exists |
| Roo Code | `.roo/skills/`, `.agents/skills/` | `~/.roo/skills/`, `~/.agents/skills/` | Yes | `.roo/rules/`, `.roorules`, `AGENTS.md` | `~/.roo/rules/` | `~/.roo` exists (editor extension, no binary) |
| Goose | `.agents/skills/` (also `.goose/skills/`, `.claude/skills/`) | `~/.agents/skills/` (also `~/.claude/skills/`) | Yes | Not confirmed in this research | Not confirmed | `command -v goose` |
| Kiro | `.kiro/skills/` | `~/.kiro/skills/` (IDE only) | Yes | `.kiro/steering/*.md`, `AGENTS.md` | `~/.kiro/steering/` | `~/.kiro` exists |
| Junie (JetBrains) | `.junie/skills/`, `.agents/skills/` | `~/.junie/skills/`, `~/.agents/skills/` | Yes | `.junie/AGENTS.md`, `AGENTS.md`, legacy `.junie/guidelines.md` | `~/.junie/AGENTS.md` | `~/.junie` exists |
| Zed | `.agents/skills/` (trusted worktrees only) | `~/.agents/skills/` | Yes | `.rules`, `AGENTS.md`, `CLAUDE.md`, `GEMINI.md` and others (first match wins) | `~/.config/zed/AGENTS.md` | `command -v zed`, or `~/.config/zed` exists |
| Aider | none | none | No evidence found | none auto-loaded; `CONVENTIONS.md` loaded via `--read` or `.aider.conf.yml` | n/a | `command -v aider` |

### What this means for the installer

- `.agents/skills/homebrew-bench/` (project) and `~/.agents/skills/homebrew-bench/` (user) is read by Codex, Cursor, Gemini CLI, Copilot, Windsurf, opencode, Amp, Roo Code, Goose, Junie and Zed. One copy covers most of the field.
- Claude Code does not list `.agents/skills/` in its location table, so it needs its own copy at `.claude/skills/` or `~/.claude/skills/`. That same path is also read by Cursor, Copilot, opencode, Amp, Goose and Cline, but Cline reads `.claude/skills/` only at project level and has no documented user-level equivalent.
- Cline at user level needs `~/.cline/skills/`. Kiro needs `.kiro/skills/` or `~/.kiro/skills/`. Neither reads `.agents/skills/` according to its docs.
- Agents with no skills support (Aider) would only get a pointer through an instructions file or `--read`. An optional fallback is a short `AGENTS.md` snippet that tells the agent to read `homebrew-bench/SKILL.md`, since `AGENTS.md` is read by nearly everyone above.
- Do not write into a directory unless the agent is detected or the user opts in. Copying into the shared `.agents/skills/` and `.claude/skills/` locations is cheap and harmless for agents that are not installed, but it still pollutes the repo.
- Frontmatter: keeping to `name` and `description` (plus optionally `license`, `compatibility`, `metadata`) is accepted everywhere. `name` must equal the folder name `homebrew-bench`. Avoid Claude-only fields (`allowed-tools`, `context`, `hooks`) in the shared skill, since stricter hosts reject unknown keys (see Claude Code notes).

## Per-agent notes

### Claude Code

- Skill locations: enterprise (managed settings directory `.claude/skills/...`), personal `~/.claude/skills/<skill-name>/SKILL.md`, project `.claude/skills/<skill-name>/SKILL.md`, nested `<subdir>/.claude/skills/...`, plugin `<plugin>/skills/<skill-name>/SKILL.md`, and skills synced from a claude.ai account. Project skills are also picked up from parent directories up to the repo root. Source: https://code.claude.com/docs/en/skills (the older docs.claude.com URL redirects here).
- `.agents/skills/` is not in that page's location table. I could not confirm whether Claude Code reads it. Treat it as unsupported.
- Frontmatter: no field is required. `description` is recommended. Documented fields include `name`, `description`, `when_to_use`, `argument-hint`, `arguments`, `disable-model-invocation`, `user-invocable`, `allowed-tools`, `disallowed-tools`, `model`, `effort`, `context`, `agent`, `background`, `hooks`, `paths`, `shell`, plus informational `metadata`, `license`, `compatibility`. Frontmatter is only read when `---` is the first line. Outside Claude Code (claude.ai uploads, Skills API) only `name`, `description`, `license`, `compatibility`, `metadata`, `allowed-tools` are allowed and other keys cause a hard error. Source: same page.
- Folder names `synced` and `anthropic-skills` are reserved. Source: same page.
- Instructions: user `~/.claude/CLAUDE.md`; project `./CLAUDE.md` or `./.claude/CLAUDE.md`; local `./CLAUDE.local.md`; rules in `.claude/rules/*.md` with optional `paths` frontmatter. `AGENTS.md` is read only when there is no `CLAUDE.md` or `CLAUDE.local.md` in the working directory or above, and needs v2.1.277 or later. Source: https://code.claude.com/docs/en/memory
- Detection: `claude --version` prints a version, and `claude doctor` prints diagnostics. The native installer places the launcher at `~/.local/bin/claude` on macOS/Linux and `%USERPROFILE%\.local\bin\claude.exe` on Windows. `~/.claude` and `~/.claude.json` hold user state, but the VS Code extension, JetBrains plugin and Desktop app also create `~/.claude/`, so the directory alone does not prove the CLI is installed. Source: https://code.claude.com/docs/en/setup
- `CLAUDE_CONFIG_DIR`: not confirmed. The pages I read did not mention it.

### OpenAI Codex CLI

- Skill locations: repo `$CWD/.agents/skills`, `$CWD/../.agents/skills` and `$REPO_ROOT/.agents/skills` (every directory from cwd up to the repo root); user `$HOME/.agents/skills`; admin `/etc/codex/skills`; plus bundled system skills. Skills sharing a name are not merged. Symlinked skill folders are followed. Source: https://learn.chatgpt.com/docs/build-skills (redirect target of https://developers.openai.com/codex/skills; the redirect was server-supplied, so I could not independently verify the host, though the original developers.openai.com URL is OpenAI's).
- Frontmatter: `name` and `description` required. Optional `agents/openai.yaml` sets UI metadata, invocation policy and tool dependencies. `policy.allow_implicit_invocation` defaults to true. A skill can be disabled with a `[[skills.config]]` entry in `~/.codex/config.toml`. Source: same page.
- Instructions: global `~/.codex/AGENTS.override.md` then `AGENTS.md` (first non-empty file only). Project: walks from the project root to cwd, one file per directory, checking `AGENTS.override.md`, `AGENTS.md`, then names in `project_doc_fallback_filenames`. Combined size stops at `project_doc_max_bytes` (32 KiB default). `CODEX_HOME` relocates the home directory. Source: https://learn.chatgpt.com/docs/agent-configuration/agents-md
- Detection: install methods are `npm install -g @openai/codex`, `brew install --cask codex`, and install scripts; the binary is run as `codex`. Source: https://github.com/openai/codex . The README did not mention `~/.codex` or `CODEX_HOME`; those come from the AGENTS.md page above. Check `command -v codex`, and honour `$CODEX_HOME` if set.

### Cursor

- Skill locations: project `.agents/skills/` and `.cursor/skills/`; user `~/.agents/skills/` and `~/.cursor/skills/`. Cursor also loads `.claude/skills/`, `.codex/skills/`, `~/.claude/skills/` and `~/.codex/skills/` as compatibility directories. Source: https://cursor.com/docs/context/skills
- Frontmatter: `name` (lowercase letters, numbers, hyphens, must match the folder) and `description` required. Optional: `paths`, `disable-model-invocation`, `icon`, `color`, `metadata`. Source: same page.
- Rules: project rules are `.mdc` files in `.cursor/rules`; a plain `.md` file there is ignored. Frontmatter fields are `description`, `globs`, `alwaysApply`. `AGENTS.md` (plain markdown, no frontmatter) is supported in the root and subdirectories. User rules are defined in the settings UI, and team rules in the dashboard. The rules page does not mention skills or legacy `.cursorrules`, so it predates or omits the skills feature. Source: https://cursor.com/docs/context/rules
- Detection: Cursor CLI installs with `curl https://cursor.com/install -fsS | bash` (Windows: `irm 'https://cursor.com/install?win32=true' | iex`) and the docs invoke it as `agent`. Source: https://cursor.com/docs/cli/overview . The desktop app has no documented detection path. `~/.cursor` is implied by the documented `~/.cursor/skills/` path but the docs do not say the directory always exists. Not confirmed whether the CLI reads rules, AGENTS.md or skills.

### Gemini CLI

- Skill locations, lowest to highest precedence: built-in, extension skills, user `~/.gemini/skills/` or `~/.agents/skills/`, workspace `.gemini/skills/` or `.agents/skills/`. Within a tier `.agents/skills/` wins over `.gemini/skills/`. Source: https://geminicli.com/docs/cli/skills/
- Frontmatter: `name` and `description`. The pages do not say explicitly which are required, and `name` should match the folder. `gemini skills install <url>` installs from a git repo and `gemini skills link .` links a local skill. Source: https://geminicli.com/docs/cli/creating-skills/
- Instructions: global `~/.gemini/GEMINI.md`; workspace and parent directories; subdirectories loaded just in time. `context.fileName` in `settings.json` can be a list such as `["AGENTS.md", "CONTEXT.md", "GEMINI.md"]`. Source: https://geminicli.com/docs/cli/gemini-md/
- Detection: binary `gemini`; install via `npm install -g @google/gemini-cli` or `brew install gemini-cli`. Source: https://geminicli.com/docs/get-started/installation/ . That page also states that for unpaid tier and Google One users, "Gemini CLI was replaced by Antigravity CLI on June 18th, 2026." I did not research Antigravity CLI, so its skill locations are unknown and worth a follow-up.
- No version-check command or settings path was documented on the install page.

### GitHub Copilot

- Skill locations: project `.github/skills`, `.claude/skills`, `.agents/skills`; personal `~/.copilot/skills`, `~/.agents/skills`. Supported surfaces: the Copilot app, Copilot CLI, cloud agent and code review on GitHub.com, and agent mode in VS Code, Visual Studio and JetBrains IDEs. A personal skill may not be available to a cloud agent. Source: https://docs.github.com/en/copilot/concepts/agents/about-agent-skills
- VS Code additionally reads `~/.claude/skills/` and offers the `chat.agentSkillsLocations` setting, which the docs say is not recommended for Copilot workflows because the CLI and app do not inherit editor settings. Source: https://code.visualstudio.com/docs/copilot/customization/agent-skills
- Frontmatter in VS Code: `name` (required, max 64, must match the directory), `description` (required, max 1024), optional `argument-hint`, `user-invocable`, `disable-model-invocation`, `context` (experimental). Source: same VS Code page. The Copilot CLI page lists `name`, `description`, `license`, `allowed-tools`. Source: https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-skills
- Instructions: `.github/copilot-instructions.md` (repo-wide); `.github/instructions/**/NAME.instructions.md` with `applyTo` globs (cloud agent and code review only); `AGENTS.md` anywhere in the repo; `CLAUDE.md` and `GEMINI.md` at the repo root. Which surface reads `AGENTS.md`, `CLAUDE.md`, `GEMINI.md` was not stated. Source: https://docs.github.com/en/copilot/how-tos/configure-custom-instructions/add-repository-instructions
- Detection: CLI installs via `npm install -g @github/copilot`, `winget install GitHub.Copilot`, `brew install --cask copilot-cli`, or `curl -fsSL https://gh.io/copilot-install | bash`. Source: https://docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/install-copilot-cli . The install page does not name the executable. The skills page uses `copilot` in its example commands, so the binary is probably `copilot`, but that is not stated. The `COPILOT_HOME` variable and a user-level instructions file path were not confirmed. For the VS Code extension, a separate check is needed (for example listing installed VS Code extensions), which is outside what these docs describe.

### Windsurf (docs now hosted under Devin Desktop)

- The docs.windsurf.com pages now redirect (307) to docs.devin.ai, and the product pages describe "Devin Desktop" and still call the assistant Cascade. Whether Windsurf the editor and Devin Desktop are the same installed product was not stated in the pages I read; I treated them as the same, which is an assumption.
- Skill locations: workspace `.devin/skills/` (preferred; legacy `.windsurf/skills/` still read); global `~/.codeium/windsurf/skills/` and `~/.config/devin/skills/`; enterprise system paths such as `/Library/Application Support/Devin/skills/` on macOS. Also discovers `.agents/skills/` and `~/.agents/skills/`, and `.claude/skills/` and `~/.claude/skills/` only if Claude Code config reading is enabled. Source: https://docs.devin.ai/desktop/cascade/skills
- Frontmatter: `name` and `description` required. Source: same page.
- Rules: global `~/.codeium/windsurf/memories/global_rules.md` (6,000 chars max, no frontmatter); workspace `.devin/rules/*.md`, fallback `.windsurf/rules/*.md`, legacy `.windsurfrules`; each workspace rule up to 12,000 chars; `trigger` frontmatter with `always_on`, `model_decision`, `glob`, `manual`; `AGENTS.md` in any directory. Source: https://docs.devin.ai/desktop/cascade/memories
- Detection: no CLI binary or install path documented in what I read. `~/.codeium/windsurf` is the documented user directory, so its presence is a reasonable hint.

### Cline

- Skill locations: project `.cline/skills/` (recommended), `.clinerules/skills/`, `.claude/skills/`; global `~/.cline/skills/` (macOS/Linux) or `C:\Users\USERNAME\.cline\skills\` (Windows). If a global and a project skill share a name, the global one wins. Source: https://docs.cline.bot/customization/skills
- Cline does not list `.agents/skills/` or `~/.claude/skills/`. For user-level installs it needs `~/.cline/skills/`.
- Frontmatter: `name` (required, must exactly match the directory name) and `description` (required, up to 1024 chars). Source: same page.
- Rules: workspace `.clinerules/` or `.cline/rules/`; also detects `.cursorrules` and `.windsurfrules`; global rules `~/Documents/Cline/Rules` (all OSes, with OneDrive variants on Windows), also `~/.cline/rules` and `~/Cline/Rules`; `AGENTS.md` in the workspace and `~/.agents/AGENTS.md` globally. Conditional rules use a `paths` glob array in frontmatter; rules without frontmatter are always active. Source: https://docs.cline.bot/customization/cline-rules
- Detection: CLI installs with `npm i -g cline` and runs as `cline`. The default config directory is not documented on the CLI overview page (only `--config` and `--data-dir` flags). The editor extension has no CLI to check. `~/.cline` is implied by the skills path. Source: https://docs.cline.bot/cline-cli/overview

### opencode

- Skills: project `.opencode/skills/`, `.claude/skills/`, `.agents/skills/` (walks up to the git worktree); global `~/.config/opencode/skills/`, `~/.claude/skills/`, `~/.agents/skills/`. Frontmatter `name` and `description` required; `license`, `compatibility`, `metadata` optional; unknown fields ignored. Source: https://opencode.ai/docs/skills/
- Instructions: project `AGENTS.md` then `CLAUDE.md` fallback; global `~/.config/opencode/AGENTS.md`, fallback `~/.claude/CLAUDE.md`. `OPENCODE_DISABLE_CLAUDE_CODE=1` turns off `.claude` support. Source: https://opencode.ai/docs/rules/
- Detection: binary name `opencode` was not stated on the pages I read; it is inferred from the project name.

### Amp

- Skills: project `.agents/skills/` and `.claude/skills/` (searched up the parent directories); user `~/.config/agents/skills/`, `~/.agents/skills/`, `~/.config/amp/skills/`, `~/.claude/skills/`, `~/.claude/plugins/cache/`. First skill found with a name wins. Frontmatter `name`, `description`, optional `mcpServers`. Source: https://ampcode.com/docs/customize/skills
- Instructions: `AGENTS.md` in cwd and parents up to `$HOME`; user `$HOME/.config/amp/AGENTS.md` and `$HOME/.config/AGENTS.md`; fallback `AGENT.md` or `CLAUDE.md`. Source: https://ampcode.com/docs/customize/agents-md
- Detection: the binary name was not confirmed from the pages I read.

### Roo Code

- Skills: global `~/.roo/skills/` and `~/.agents/skills/`; project `.roo/skills/` and `.agents/skills/`; mode-specific variants `skills-{modeSlug}`. `.roo/` outranks `.agents/`. Frontmatter `name` (must match directory) and `description`. Source: https://roocodeinc.github.io/Roo-Code/features/skills (docs.roocode.com redirects to this host).
- Instructions: `.roo/rules/`, `.roorules`, global `~/.roo/rules/`, and `AGENTS.md` (fallback `AGENT.md`) in the workspace root, controlled by `roo-cline.useAgentRules`. Source: https://roocodeinc.github.io/Roo-Code/features/custom-instructions
- Detection: it is an editor extension. No CLI is documented in what I read.

### Goose

- Skills: global `~/.agents/skills/`, project `.agents/skills/`, plugin skills under `~/.agents/plugins/<name>/`. Also discovers `.goose/skills/`, `.claude/skills/`, `~/.claude/skills/`. Frontmatter `name` and `description` required. Source: https://goose-docs.ai/docs/guides/context-engineering/using-skills/ . The older block.github.io docs URL returned 404, so I used goose-docs.ai; I could not verify the host's ownership from the page, so treat this source as slightly less certain.
- Instructions file (such as `.goosehints`): not confirmed in this research.

### Kiro

- Skills: workspace `.kiro/skills/`; global `~/.kiro/skills/` (IDE only). Frontmatter `name` (must match folder) and `description` required. Custom agents do not load skills unless `skill://` URIs are added to `resources`. Source: https://kiro.dev/docs/skills/
- Steering: `.kiro/steering/` and `~/.kiro/steering/`, with `inclusion` modes `always`, `fileMatch`, `manual`, `auto`. `AGENTS.md` is read from `~/.kiro/steering/`, the workspace root and subdirectories. Source: https://kiro.dev/docs/steering/
- Detection: not documented in what I read.

### Junie (JetBrains)

- Skills: project `<projectRoot>/.junie/skills/<skill-name>/`; user `~/.junie/skills/<skill-name>/`; also scans `<projectRoot>/.agents/skills/` in a trusted project and `~/.agents/skills/`, extension skills, and custom folders via `--skill-location` or `skill-locations` in `config.json`. Frontmatter: `name` required, `description` optional. The page says Junie detects `.claude/skills/` folders and suggests importing them, but does not list `.claude/skills/` as scanned. Source: https://junie.jetbrains.com/docs/agent-skills.html
- Guidelines: `.junie/AGENTS.md` preferred, otherwise root `AGENTS.md` with `.junie/rules/*.md`; legacy `.junie/guidelines.md`; global `~/.junie/AGENTS.md`. This detail came from a search result summary of https://junie.jetbrains.com/docs/guidelines-and-memory.html and I did not open that page directly, so it is lower confidence.
- A JetBrains AI Assistant page listed Junie skills support as unsupported (a dash in its table) while the Junie docs describe it; I did not resolve that conflict. The Junie docs are the owning source.

### Zed

- Skills: user `~/.agents/skills/`; project `<worktree>/.agents/skills/` (trusted worktrees only). Frontmatter `name` and `description` required, optional `disable-model-invocation`. Source: https://zed.dev/docs/ai/skills
- Instructions: personal `~/.config/zed/AGENTS.md` (Windows `%APPDATA%\Zed\AGENTS.md`). Project: first match from `.rules`, `.cursorrules`, `.windsurfrules`, `.clinerules`, `.github/copilot-instructions.md`, `AGENT.md`, `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`. Source: https://zed.dev/docs/ai/instructions
- Detection: not documented in what I read.

### Aider

- Does not auto-load an instructions file. A conventions file is loaded with `/read CONVENTIONS.md`, `aider --read CONVENTIONS.md`, or a `read:` key in `.aider.conf.yml`. The page does not mention `SKILL.md`, so support could not be confirmed and is presumed absent. Source: https://aider.chat/docs/usage/conventions.html

## Gaps and open items

- Not confirmed: Claude Code reading `.agents/skills/`; `CLAUDE_CONFIG_DIR`; Copilot user-level instructions file and `COPILOT_HOME`; Cursor CLI behaviour for rules and skills; Cline CLI default config directory; binary names for opencode, Amp, Kiro, Zed, Windsurf.
- Not researched: Antigravity CLI (which the Gemini CLI install page says replaced Gemini CLI for unpaid and Google One users on 2026-06-18), Continue, Devin CLI, JetBrains AI Assistant, Sourcegraph Cody, Warp, Replit.
- Not tested: nothing was run. All paths are documented claims, not verified behaviour.
- Detection is design advice, not vendor guidance. For robustness, an installer should check for both a binary on `PATH` and the user config directory, and offer a flag to force specific targets.
