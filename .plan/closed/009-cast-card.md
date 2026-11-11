---
id: 009
title: Cast card
type: task
mode: AFK
status: closed
blocked_by: [006]
claimed: false
---

## Question

Build the cast card from [Live spell casting prompts](../closed/003-live-spell-casting-prompts.md) ([ADR 0022](../../docs/adr/0022-live-spell-casts-reuse-bench-action.md)): a read-only `bench spell <spell-id> [--caster <id>] [--slot N]`.

## Notes

- Skills: `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- It prints level, casting time, concentration, save or attack kind, the effects resolved at the cast level (scaled dice, target count), and every agent clause and the upcast clause verbatim.
- With `--caster` against a live encounter, it adds that caster's save DC or attack bonus and remaining slots, and reads homebrew entries from state. Without an encounter, it reads `--spell` files.
- It states how many target ids `bench action` takes. The engine's refusal of a wrong target count states the same number, from the same function.
- An unknown id errors and names the missing file.
- Reuse the scaling code the engine runs, so the card and the cast cannot disagree.
- Tests: a homebrew save spell at its level and upcast; a cantrip at a higher caster level; caster numbers from a stat block; the target-count sentence matches the engine's refusal; unknown id.
- Add the command to `bench help --json`.

## Resolution

Built in engine 0.1.40. `bench spell <spell-id> [--caster <id>] [--slot N]` prints a cast card as JSON with a `text` rendering: level, cast level, casting time, concentration, save or attack, the effects at that level, the target count, every agent clause and the upcast clause. Effects come from `engine::cast_effects`, the function `cast_spell` and batch tactics now share. With `--caster` or `-e` it reads the running encounter's spell book and the caster's save DC, attack bonus, slots left and casting actions; otherwise the SRD and `--spell` files. An unknown id is `not_found` and names the files read. `TargetCount::of` gives the count, and live `bench action` refuses a cast with a wrong count using the same sentence. The check is live only, so batch numbers are unchanged. A self spell now refuses a target id, and the live-mode guide says so. Single-target save spells read "1 or more" until the target-cap fog is decided. With an explicit `--slot`, the card refuses a homebrew upcast that `bench action` would refuse. Decisions are in [the decision report](../reports/009-decisions.md). Haiku audit: APPROVE in round 3.
