---
id: 015
title: Rounds per round for spell durations
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

`rounds` durations lose one per creature's turn, so Shield and Haste last one turn. Should spell durations count per round, behind a recorded setting as `long_durations` does, and what does that do to batch cells and spell lift?

## Notes

- Grill with `grilling` and `domain-modeling`.
- Background: the archived map's Rounds per round entry ([archived map](../archive/homebrew-bench-map.md)) and [Durations longer than a round](../closed/109-durations-longer-than-a-round.md), which left the `Rounds` branch alone.
- Decide before the spell baseline is committed. A change afterward moves Shield and Haste lift and forces a regenerate.
- If the answer is a change, create its build ticket and make it block [Spell baseline](016-spell-baseline.md).

## Resolution

Yes. A `rounds: N` duration ends when the creature that applied it starts its Nth turn after it landed, behind a recorded `settings.round_durations`.

- **Premise.** Haste is not affected: its modifiers last for concentration and its lethargy uses `until_end_of_next_turn`. The SRD effects that use `rounds` are Shield and Sleep's Incapacitated. Homebrew entries may use it for anything.
- **Meaning.** The rules read "1 round" as "until the start of your next turn", which is Shield's text. Today Shield cast as a reaction ends with the attacker's turn, so later enemies before the caster's turn see normal AC. A round-clock count (as long durations use) would end it at the top of the round instead of at the caster's turn, and a new `until_start_of_next_turn` unit would leave homebrew `rounds` wrong.
- **Count.** `Rounds(n)` loses one each time its source's initiative slot comes up, including when the source is skipped as down, and expires at 0. When the source is None or not in the initiative order, the round clock is used (`started_round + N`).
- **Old saves.** `settings.round_durations` gates it as `long_durations` does: a save without it keeps the per-turn decrement and refolds as recorded, except for Sleep's expiry event (see Sleep below). `bench start` and batch set it true.
- **Sleep.** Its Incapacitated becomes `until_end_of_next_turn`, as the spell says, with no gate. A refold reads SRD entries from the embedded data, so an old Sleep save moves when that condition's expiry event fires. Unconscious lands with it and covers it, so hit points and end states hold, and the build tests that. A spell-specific engine branch was rejected, and so was keeping a rules-wrong entry.
- **Batch cells and spell lift.** The standard party carries Shield and Sleep, so the build regenerates the curve and the spell panel and states what moved. Policy stays v5: it already reads `rounds` as rounds. It still scores Shield only on the triggering hit, a known conservative bias. Spell lift moves for Shield's level 1 row and for homebrew `rounds` spells, which is why the build blocks the spell baseline.
- **Docs.** A "Round duration" glossary term, a paragraph in ADR 0005 and an intake mapping in `spell-intake.md`. "Until the end of your next turn" on another creature has no unit. It is written `rounds: 1` and flagged as inferred.
- **Surfaced.** The engine never runs `delayed` effects, so Acid Arrow and Vitriolic Sphere lift reads low. This and caster-anchored end-of-turn durations join Not yet specified.

Decided in auto mode by a grilling subagent over three rounds and fourteen questions. It accepted every recommendation and added the downed-source and same-turn tick checks. The auditor's finding that a refold reads SRD data reopened the Sleep question, and the third round settled it. The build is [Build rounds per round](../tickets/020-build-rounds-per-round.md), which blocks [Spell baseline](../tickets/016-spell-baseline.md).
