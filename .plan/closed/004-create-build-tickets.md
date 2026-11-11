---
id: 004
title: Create build tickets and append them to the map
type: grilling
mode: HITL
status: closed
blocked_by: [001, 002, 003]
claimed: false
---

## Question

From the decisions closed on this map, what are the implementation tickets, in what order, and what does each one deliver?

Decide:

- The slices, each small enough for one session and written so that `tdd` can drive it
- The order and the blocking edges between them
- Which fog items from Not yet specified become build tickets, and which stay open

Then create the build tickets in `tickets/` with the next free number, and append them to this map's Frontier.

## Notes

- Grill with `grilling` and `domain-modeling`.
- Build tickets are the output of this ticket. Each one names the skills a build session calls: `tdd`, `codebase-design`, `domain-modeling`, `code-review`, and `writing-for-agents` when it edits a skill.
- Blocked by 001, 002 and 003. Do not start until all three have closed.

## Resolution

Thirteen build tickets and one grilling ticket, numbered 005 to 018 and appended to the map. Decided in auto mode by the grilling subagent, which took every recommendation and added three refinements: Spell entry fields also declares the `agent_clauses` field that `effects.json` carries and `SpellEffect` drops; Model more SRD cantrips is blocked by Spell entry fields, because each new cantrip needs a `casting_time`; and the Spell lift runner must pick an interval the report can state in one sentence.

Slices and blocking edges:

- [Spell entry fields](../tickets/005-spell-entry-fields.md): `casting_time`, declared agent clauses, homebrew fields, schema export. Unblocked.
- [Spell book](../tickets/006-spell-book.md): homebrew entries in encounter state, looked up before the SRD at the three `catalog()` call sites; load refusals; `--spell` on `bench schema validate`. Blocked by 005.
- [Live start with homebrew spells](../tickets/007-live-start-with-homebrew-spells.md): `bench start --spell` and the start-time checks. Blocked by 006.
- [Live cast log for homebrew clauses](../tickets/008-live-cast-log-for-homebrew-clauses.md): clause events after the window. Blocked by 007.
- [Cast card](../tickets/009-cast-card.md): `bench spell`. Blocked by 006.
- [Skill: homebrew spell intake and live casting](../tickets/010-skill-homebrew-spell-intake-and-live-casting.md). Blocked by 007, 008, 009.
- [Batch homebrew clause resolution](../tickets/011-batch-homebrew-clause-resolution.md): `bench batch --spell`, clause assumptions, entry fingerprints. Blocked by 006.
- [Control party and spell panel](../tickets/012-control-party-and-spell-panel.md). Unblocked.
- [Spell lift runner](../tickets/013-spell-lift-runner.md): swap, rows, `bench batch --test-spell`. Blocked by 006, 011 (the batch `--spell` flag), 012.
- [Model more SRD cantrips](../tickets/014-model-more-srd-cantrips.md): only three are modeled, so every cantrip row would be thin. Blocked by 005.
- [Rounds per round for spell durations](../tickets/015-rounds-per-round-for-spell-durations.md): grilling. Unblocked.
- [Spell baseline](../tickets/016-spell-baseline.md): `bench baseline generate --spells` and `check --spells`. Blocked by 013, 014, 015.
- [Spell lift report](../tickets/017-spell-lift-report.md). Blocked by 011, 016.
- [Skill: batch spell testing](../tickets/018-skill-batch-spell-testing.md). Blocked by 010, 017.

Choices and reasons:

- Thin vertical slices over three big ones (intake, live, batch). Each fits one session and has a narrow seam for `tdd`. Spell book stays one ticket because `catalog()` has only three call sites. Each later ticket adds `--spell` to the command it touches.
- The control transform goes with panel selection, because ADR 0020 defines the panel by the control run's win rate.
- Lift runs through `bench batch --test-spell` and the SRD table through `bench baseline --spells`, so they reuse batch's seeds, assumptions, replay and staleness code. Plain `bench baseline check` keeps reporting the monster curve only.
- Fight count and interval method are left to the Spell lift runner, recorded in ADR 0020. Paired seeds are fixed.
- Fog: Rounds per round becomes a grilling ticket that blocks the spell baseline. It moves Shield and Haste lift, and deciding it after the baseline is committed forces a regenerate. The other five stay in Not yet specified. Each already has a defined reading (documentary caps, "not measured", SRD clauses read from `effects.json`) and none changes the committed numbers. Counter pricing and SRD agent clauses on cast are the next candidates if the baseline review shows they matter.
- Engine tickets name `tdd`, `codebase-design`, `domain-modeling`, `code-review`. Skill tickets name `writing-for-agents`, `domain-modeling`, `code-review`. The cantrip data ticket names `tdd`, `code-review`. Build tickets are `task`/AFK in the format of the archived map's [Build durations longer than a round](113-build-durations-longer-than-a-round.md).
