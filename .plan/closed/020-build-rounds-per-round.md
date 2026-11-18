---
id: 020
title: Build rounds per round
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Build the source-anchored `rounds` duration decided in [Rounds per round for spell durations](../closed/015-rounds-per-round-for-spell-durations.md): with `settings.round_durations` on, a `rounds: N` modifier or condition ends when the creature that applied it starts its Nth turn after it landed.

## Notes

- Skills: `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- **Setting.** `settings.round_durations` on `EncounterSettings` and `StartSpec`, `serde(default, skip_serializing_if = "Not::not")`, as `long_durations` is. `bench start` and batch set it true. A save without it loads false and keeps the per-turn decrement in `decrement_durations`, so it refolds as recorded.
- **Count.** With the setting on, `decrement_durations` skips `Rounds`. Instead, `Rounds(n)` loses one each time its source's initiative slot comes up, and the effect expires at 0. No new field. An effect applied during its source's own turn first ticks at the source's next turn start, so Shield cast on its caster's turn and Shield cast as a reaction both last until the caster's next turn starts.
- **Downed source.** `start_next_turn` skips Dead, Unconscious and Stable creatures with a `forfeit` event, so their turn never starts. The tick runs as the initiative passes the source's slot, before the forfeit skip, so a downed source still ticks.
- **No source.** When `source` is None or names a creature not in the initiative order (removed, or never in it), fall back to the round clock: expire when `state.round >= started_round + N`, checked where `expire_long_durations` runs.
- **Sleep.** In `data/spells/effects.json`, Sleep's Incapacitated becomes `until_end_of_next_turn` ("until the end of its next turn"), with no gate, following the See Invisibility precedent (ticket 113). A refold reads SRD entries from the embedded data, so an old save that holds Sleep moves its Incapacitated `condition_expire` event even with the setting off. Unconscious lands with it, includes Incapacitated and also ends on damage, so hit points and end states should not move. Add a test that an old Sleep save with the setting off ends with the same hit points and end states as recorded. Check the refold tests for strict event-log comparisons that touch Sleep, and update any that do on purpose.
- **Policy.** No policy change, so it stays v5. Shield is still taken only when it turns the triggering hit into a miss, and its +5 on later turns is not scored. That is a known conservative bias on Shield, not a defect.
- **Regenerate.** Regenerate `data/baseline/curve.json` (the standard party carries Shield and Sleep) and `data/baseline/spell-panel.json` with the setting on, at `--threads 4` or lower. If control win rates moved, re-pick the panel fixtures. State which cells and fixtures moved in the changelog.
- **Docs.** Add a "Round duration" term to `CONTEXT.md`. Add a paragraph to ADR 0005 next to the long-durations one. In `spell-intake.md`, map "1 round" and "until the start of your next turn" to `rounds: 1` and "until the end of its next turn" (the target's) to `until_end_of_next_turn`. "Until the end of your next turn" on another creature has no unit: write `rounds: 1` and flag it as inferred, one turn short.
- **Tests.** Shield as a reaction lasts through the turns of later enemies until the caster's turn starts. A self-applied `rounds: 1` on the caster's own turn survives that turn's end. A downed source's slot still ticks. A source that is None or not in the order uses the round clock. An old save with the setting false refolds as recorded.
- **Version.** Engine 0.1.45 with a changelog entry.

## Resolution

Built. With `settings.round_durations` on, a `rounds: N` modifier or condition loses one each time its source's initiative slot comes up, including a downed source's skipped slot, and ends at 0. One with no source in the initiative order ends on the round clock, N rounds after it landed. `bench start` and batch set the setting. Saves from before it keep the per-turn decrement.

- **Sleep.** Its Incapacitated is `until_end_of_next_turn` in the effects file. A real 0.1.45 Sleep save, committed as a test fixture, refolds to the same hit points and end states.
- **Regenerated.** The curve moves 38 of 6,820 fixture cells on nine fixtures (Archmage, Lich, Mage, Hydra, Ettin, Water, Air and Earth Elemental, Invisible Stalker), with no pooled band change. The spell panel re-picks its group fixtures at party levels 3, 5, 7, 9, 11, 13, 15 and 17.
- **Version.** Engine 0.1.46, not 0.1.45, because 0.1.45 shipped with ticket 019.
- **Policy.** Stays v5. Shield's +5 on later turns is still not scored.

Council approved in round 1 (claude 8, gemini 10, gpt 9). Decisions are in [the decision report](../reports/020-decisions.md).

Timing: wall 44m, active 43m, user 0s, wait 8s, blocked 0s | implement 41m, audit 2m (1 round) | tools 70 (2 subagent) | limits 0, compactions 0, handoffs 0 | ctx peak 156k | cost $2.35 sonnet-5-5 + $0.02 haiku-5-5
