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

The interface ticket fixed `install.sh` and `install.ps1`, which download the release binary into `~/.bench/bin/`. Use the agent skill locations findings. Consult `grilling` and `domain-modeling`.

## Resolution
