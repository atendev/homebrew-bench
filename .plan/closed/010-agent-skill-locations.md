---
id: 010
title: Agent skill locations
type: research
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Where do Claude Code, Codex, Cursor and other common coding agents load skills or instructions from, at project and user level? What file format does each expect, does each support the `SKILL.md` convention, and how could an installer detect that an agent is present?

## Notes

Primary sources only: each agent's official documentation. Use the `research` skill and write the findings as a Markdown file with a table of agent, paths, format and detection method. This feeds Packaging. Do not launch the research agent until the current planning work is committed.

## Resolution

Resolved by a research agent on 2026-10-27. Findings live in `docs/research/agent-skill-locations.md`, with a table of 15 agents, their project and user paths, and detection ideas, all cited to official docs and caveated as a 2026-10-24 snapshot.

One copy at `.agents/skills/homebrew-bench/` (project) or `~/.agents/skills/homebrew-bench/` (user) covers most of the field. Claude Code needs its own copy under `.claude/skills/` (confirmed reading `.claude`, `.agents` unconfirmed). Cline at user level needs `~/.cline/skills/` and Kiro needs `.kiro/skills/` or `~/.kiro/skills/`; neither reads `.agents/skills/`. A short `AGENTS.md` pointer is the fallback for agents without skills support (Aider). Frontmatter stays to `name` and `description`, with `name` equal to the folder name, since stricter hosts reject unknown keys. Detection is inference, not vendor guidance: check a binary on PATH plus the user config directory, and never write into an undetected agent's directory.

Surfaced: Gemini CLI's install page says it was replaced by Antigravity CLI for unpaid tiers on 2026-06-18; Antigravity's skill locations are unresearched. Left as a caveat on the Packaging ticket rather than a new ticket. Unblocks Packaging, SRD data and skill install.
