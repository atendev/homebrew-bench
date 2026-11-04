---
id: 072
title: Document saving-throw condition sources in live mode
type: task
mode: AFK
status: open
blocked_by: [069]
claimed: false
---

## Question

Add the live-mode line that ticket [069](../closed/069-build-saving-throw-condition-sources.md) asked for. The builder's edit to `.agents/skills/homebrew-bench/references/live-mode.md` was held by the shell guard and not made.

## Notes

In `.agents/skills/homebrew-bench/references/live-mode.md`, under step 2 after the bullet that ends "Multiattack, spells and saves all go through this command.", add one bullet:

> The engine fails Strength and Dexterity saves on its own, with no die and no prompt, for a Paralyzed, Petrified, Stunned or Unconscious creature and a downed PC, and gives a Restrained creature Disadvantage on Dexterity saves.

Docs only: no version bump, no tests. Consult `writing-for-agents`.

## Resolution
