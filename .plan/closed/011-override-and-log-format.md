---
id: 011
title: Override mechanics, log format and public view
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

What path syntax does `bench override` use to set engine state? How do overrides, user-entered rolls, `bench roll` results, undo reversals, pending prompts and agent-judgment labels read in the log? How does the log read to a human in live mode and in a sampled batch log? What does the optional public view hide, and how are wounded states worded?

## Notes

Inputs are closed: the interface ticket fixed `bench override` with a required reason and a tamper hash on every log entry. The state-model ticket fixed the state fields. The live-mode ticket fixed undo, `bench roll`, roll sources and the events list in `docs/adr/0004-whole-actions-with-pending-prompts.md`. Consult `grilling` and `domain-modeling`.

## Resolution

Decided with the user on 2026-10-26. Recorded in `docs/adr/0006-override-paths-and-log-format.md`. New terms (public view, wounded state) are in `CONTEXT.md`.

- Override syntax: dotted domain paths over the state model (`goblin-2.hp 14`, `mia.conditions '["prone"]'`). Scalar values go in bare, structured values as JSON, and `--clear` removes a value. The engine validates paths against the schema family and reports field-path errors. Every state field is a legal target except the log, ids, hashes and engine version.
- Roster changes are not overrides. `bench add <statblock> --side <side>` rolls initiative, inserts at the rolled index (the combatant acts from next round if its slot has passed), and takes `--band` or `--engaged-with`, defaulting to near its own side. `bench remove <id> --reason` mirrors it.
- Log: one plain-text file per encounter, round and turn headers, one line per event taken from the engine's ready-made lines. Uppercase tags for OVERRIDE (with reason), UNDO (naming the reverted line), AGENT JUDGMENT, PROMPT (asked and answered, including declined windows), ADD and REMOVE. User-entered dice marked inline as (player dice). House rules and standing assumptions name themselves wherever they change an outcome.
- Batch log: same rendering as live, opening with a header block listing every standing assumption. No player dice, normally no undo.
- Public view: `bench log --public`, a filter over the same file, and the digest follows the same rules. It hides monster hit points, resources, recharge and per-round budgets, replaces hit points with wounded states, keeps the fact but drops the detail on lines touching hidden fields, and shows monster AC and the player side in full.
- Wounded states: unhurt at full, wounded below full, bloodied at half or less, at death's door at a quarter or less. Fixed vocabulary.
- Surfaced: nothing new. Unblocks House rule template and folder; Report format still waits on Standard party build and CR baseline.