---
id: 018
title: House rule template and folder
type: grilling
mode: HITL
status: closed
blocked_by: [011]
claimed: false
---

## Question

What sections does a house rule file have, what is the folder name, and how does the agent find and read house rules? How does the log name a house rule when the agent applies it?

## Notes

House rules run as prose through overrides in phase 1. Scripted house rules (Lua through mlua) are out of scope on the map, and the format should leave room for them. Consult `grilling` and `domain-modeling`.

## Resolution

Decided with the user on 2026-10-27. Recorded in `docs/adr/0009-house-rule-files-with-slug-tags.md`. `CONTEXT.md` now defines the house rule slug.

- Location: two layers, `~/.bench/house-rules/` for rules that follow the user everywhere and `./.bench/house-rules/` for one project or campaign. The project layer wins on a same-named slug. This replaces the repo-root folder named in Rules baseline, which assumed a clone; the install script path has none. The repo's own `.gitignore` now ignores `.bench/`.
- Identity: the file name without `.md` is the rule's slug and its id in the log. Renaming a file renames the rule; old logs keep the old name.
- Template: frontmatter with `status` (active or disabled), `kind` (gap or change) and `engine` (reserved, `none` in phase 1), then a title and four sections: Rule (the user's confirmed wording, verbatim), Applies when, How to apply (the override recipe), Notes (optional).
- Loading: the agent reads every file in both layers at session start. Past about ten rules, the skill has it read only title, `kind` and Applies when, and open a full file when it applies. No listing command in phase 1.
- Log naming: `bench override` and `bench roll` take `--rule <slug>`, which the engine writes into the log line as `[house rule: slug]`. The engine checks the slug against the file names in both layers and rejects an unknown one, naming the layers it searched. It reads file names only, never contents. Batch reports list rules by counting these tags.
- Per-run switches: none in the engine. The agent chooses which rules apply to a run from the user's request, and the report lists the rules that were active. `status: disabled` turns a rule off permanently.
- Scripting: Lua through mlua stays undecided. Phase 1 is prose only, and the reserved `engine:` field is the room for later modeling. Revisit it in the map that adds scripted rules.
- Surfaced: nothing new. The skill ticket owns the session-start wording and the confirm-before-write flow already set in Rules baseline.
