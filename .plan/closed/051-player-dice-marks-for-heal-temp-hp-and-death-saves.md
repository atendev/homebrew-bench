---
id: 051
title: Player-dice marks for heal, temporary hit point and death save events
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Mark heal, temporary hit point and death save events `(player dice)` in ask mode when every die behind them came from a face the user entered, as attack and damage events already are.

## Notes

These dice already go through the pending prompt, but their events carry no `player_dice` flag, which `CHANGELOG.md` records as not done yet. Follow the rule ticket 041 set: a line is marked only when every die behind it was user-entered. Add the roll fields these events need, keep the log format from ADR 0006, and update the changelog line.

## Resolution

Done in commit 0f18107 on branch `ticket-051-player-dice-marks`.

- `heal` and the `revive` a heal triggers read one mark from the healer's dice, so every target of a multi-target heal shares it.
- `Effect::TempHp` used to emit no event. It now logs `temp_hp` per target, `<target> gains <N> temporary HP`, marked from the granter's dice. The public view hides the amount for monsters. Batch fight logs show the new line too, and batch results and dice do not change.
- `death_save` events carry their d20 in `roll`, so the line ends in `[d20 N]`, and they are marked when the face was entered. The "suffers N death save failure(s) from the hit", `stabilize` and `dies` events stay unmarked, because no die stands behind them.
- Replay is unaffected, since `game_hash` leaves out the log.
- Reports: [requirements](../reports/051-requirements.md), [implementation](../reports/051-implementation.md). The haiku audit approved on round two, after the revive mark and the batch-log changelog note were added.
