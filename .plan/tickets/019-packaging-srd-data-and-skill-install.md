---
id: 019
title: Packaging, SRD data and skill install
type: grilling
mode: HITL
status: open
blocked_by: [010]
claimed: false
---

## Question

Is the SRD data embedded in the binary or shipped as files next to it? How do the skill files get installed into each agent, and does `install.sh` handle it? What does the release archive contain?

## Notes

The interface ticket fixed `install.sh` and `install.ps1`, which download the release binary into `~/.bench/bin/`. Use the agent skill locations findings (`docs/research/agent-skill-locations.md`): `.agents/skills/` covers most agents, Claude Code needs `.claude/skills/`, Cline and Kiro need their own paths, and detection is binary-plus-config-dir inference. Open caveat from the findings: Antigravity CLI (which replaced Gemini CLI for unpaid tiers in June 2026) is unresearched. The release archive must carry the SRD attribution notice; see License and attribution notice. Consult `grilling` and `domain-modeling`.

## Resolution
