---
id: 051
title: Player-dice marks for heal, temporary hit point and death save events
type: task
mode: AFK
status: open
blocked_by: []
claimed: true
---

## Question

Mark heal, temporary hit point and death save events `(player dice)` in ask mode when every die behind them came from a face the user entered, as attack and damage events already are.

## Notes

These dice already go through the pending prompt, but their events carry no `player_dice` flag, which `CHANGELOG.md` records as not done yet. Follow the rule ticket 041 set: a line is marked only when every die behind it was user-entered. Add the roll fields these events need, keep the log format from ADR 0006, and update the changelog line.

## Resolution
