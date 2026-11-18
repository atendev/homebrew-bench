# Decisions: Build rounds per round (ticket 020)

Sources: the ticket's Notes, decided in ticket 015, and choices made while building.

## Decisions made while building

1. **Where the tick runs.**
   - Question: where does a source's turn count tick?
   - Options: in `finish_turn` at the end of each turn, or in `start_next_turn` as the initiative passes each slot.
   - Choice: `start_next_turn`. It collects each skipped slot plus the slot that starts, and ticks them in order before the next turn's start-of-turn effects. A downed source's slot ticks at its `forfeit` event.
   - Made by: implementer, following the ticket's "before the forfeit skip" note.

2. **Effects applied on the source's own turn.**
   - Question: does a self-applied `rounds: 1` survive the rest of that turn?
   - Choice: yes. The tick runs at slot passes only, and the source's own slot does not pass until its next turn starts.
   - Made by: implementer, following the ticket.

3. **No source.**
   - Question: how do sourceless effects and effects with a source outside the order end?
   - Choice: `expire_long_durations` now also runs when `round_durations` is on and treats `Rounds(n)` as `started_round + n`. It still returns early only when both settings are off.
   - Made by: implementer, following the ticket.

4. **Sleep.**
   - Question: gate Sleep's change or change the data for everyone?
   - Choice: change the data. Incapacitated becomes `until_end_of_next_turn`, following See Invisibility. A refold reads SRD entries from the embedded data, so the change cannot be gated.
   - Made by: ticket 015, council round three.

5. **Old Sleep save.**
   - Question: how to test that an old Sleep save refolds to the recorded outcome?
   - Options: build one by hand, or record a real save with the old engine.
   - Choice: record one. A throwaway worktree at the previous commit ran a caster casting Sleep on a goblin, a striker killing it and four end-turns. Its state is committed as `bench-core/tests/data/engine-0.1.45-sleep-state.json`, and the worktree was removed. The test folds its history with the new data and compares hit points and end states.
   - Made by: implementer.
   - Note: the test pins hit points and end states. The event log does move, since Incapacitated now expires at the goblin's own end of turn.

6. **Engine version.**
   - Question: the ticket says 0.1.45, but 0.1.45 shipped with ticket 019.
   - Choice: 0.1.46.
   - Made by: implementer. Master should confirm.

7. **Regenerated baselines.** Curve and panel were regenerated at `--threads 4` with the release binary. The curve moves 38 of 6,820 fixture cells on nine fixtures: the Archmage, Lich, Mage, Hydra, Ettin, Water, Air and Earth Elemental, and Invisible Stalker. No pooled cell changes its band. The panel re-picks its group fixtures at party levels 3, 5, 7, 9, 11, 13, 15 and 17. The single picks stay except for win counts at the Mage (level 3) and Archmage (levels 7, 9). The changelog states these.

## Known limits, not defects

- Policy stays v5. Shield is scored only on the triggering hit, and its +5 on later turns is not scored.
- "Until the end of your next turn" on another creature is written `rounds: 1` and flagged as one turn short.

## Audit

Council APPROVE in round 1: claude 8, gemini 10, gpt 9.

- Fixed: the ADR said the tick runs "before a downed source is skipped". It now says it ticks when the initiative passes the slot, including a skipped one (claude seat, minor). The fix was wording only, so no seat re-checked it.
- Declined: the intake note calls "until the end of your next turn" on another creature "one turn short". It is the ticket's own wording and the entry is flagged as inferred (claude seat, minor).
- Declined: the diff file left in the checkout was removed before the commit (claude seat, minor, done).
