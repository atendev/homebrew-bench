---
id: 118
title: Cast card
type: task
mode: AFK
status: open
blocked_by: [115]
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

<!-- Filled in at close. -->
