---
id: 119
title: "Skill: homebrew spell intake and live casting"
type: task
mode: AFK
status: open
blocked_by: [116, 117, 118]
claimed: false
---

## Question

Write the agent steps for [Homebrew spell intake rules](../closed/002-homebrew-spell-intake-rules.md) and [Live spell casting prompts](../closed/003-live-spell-casting-prompts.md) ([ADR 0021](../../docs/adr/0021-homebrew-spell-entries-written-by-the-agent.md), [ADR 0022](../../docs/adr/0022-live-spell-casts-reuse-bench-action.md)) into the `homebrew-bench` skill.

## Notes

- Skills: `writing-for-agents`, `domain-modeling`, `code-review`.
- Intake, as a new reference beside `batch-mode.md`: write `./.bench/spells/<id>.json`, fill every field, refuse the four refusal cases with their messages (the no-combat-effect message says live mode can still play the spell as agent judgment), validate, and show the review table (every field, read or inferred, every clause verbatim, every flag) until the user confirms. An inferred level needs the user's confirmation.
- Intake flags, each with a `path` and a `note`, none blocking: a value inferred rather than read (save ability, damage type, duration, target count, level, concentration, cast time); wording with no standard reading; damage outside the DMG spell damage guideline (the note names it); a cantrip with no damage growth; an upcast with no clear per-level amount; a bonus action or reaction cast time.
- A fixed number in the text (a DC or attack bonus) is flagged and left out. It becomes an agent clause only if the user keeps it at review. A reaction trigger outside the trigger vocabulary becomes an agent clause.
- Live mode (`live-mode.md`): add the cast action to the stat block and show it before `bench start`; pass `--spell` at start; show the cast card before the user picks target and slot; reaction-time homebrew spells are cast out of turn with `bench action`; which source to read for a spell's clauses (the log for homebrew, `effects.json` for SRD).
- Link the intake reference from `SKILL.md`. Use the glossary terms Spell entry, Intake flag and Cast card from `CONTEXT.md`.

## Resolution

<!-- Filled in at close. -->
