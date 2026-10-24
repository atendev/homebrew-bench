---
id: 002
title: Baseline ruleset, 2014 or 2024
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

Does the engine implement 2014 rules (SRD 5.1) or 2024 rules (SRD 5.2) first, and does it ever support both?

## Notes

The old reason to prefer 2014 was structured multiattack. That no longer holds: both editions have every multiattack structured. 2024 adds `bonus_actions` (89) and `gear`, and has fewer prose special abilities (354 vs 551). Homebrew people paste will be a mix of both. Monster CR guidelines for the comparison also differ by edition.

## Resolution

Build the engine on 2024 rules (SRD 5.2). It is the main version of the game, so intake assumes pasted homebrew is written for 2024. 2014 homebrew isn't a supported input. The SRD 5.1 data stays in `data/srd/2014/` for reference only.

Open follow-up: the CR comparison needs a 2024 baseline. It isn't confirmed that the 2024 books have a stats-by-CR table like the 2014 DMG. The fallback is to derive one from the SRD 5.2 monsters.
