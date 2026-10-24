---
id: 006
title: Skill and repo name
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

What is the skill called, and does the repo use the same name?

## Notes

The skill name is the command people type, so short kebab-case. The current folder name is homebrew-bench. The name should fit both the batch bench and the live DM mode.

Already fixed by "How the agent talks to the engine": the binary is `bench` and its folder is `.bench/`. The user likes keeping those whatever the skill is named.

## Resolution

Decided with the user on 2026-10-24.

- The skill is named `homebrew-bench`. It does not say "monster", so items and spells can reuse the name in later maps. Live mode is not in the name. The skill description carries it: test homebrew by simulation, or DM a combat live.
- The repo uses the same name, `homebrew-bench`. The binary stays `bench` and its folder stays `.bench/`. So: clone `homebrew-bench`, run `bench`.
- Phase 1 ships one skill with two mode sections, batch and live. The agent loads the shared engine basics once and the matching mode section on demand. Revisit only if the skill file grows too large to load.
