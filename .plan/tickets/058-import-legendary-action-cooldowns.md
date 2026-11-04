---
id: 058
title: Import legendary action cooldowns
type: task
mode: AFK
status: open
blocked_by: []
claimed: false
---

## Question

Make the SRD importer set `usage: cooldown_until_next_turn` on legendary actions whose prose says the creature "can't take this action again until the start of its next turn".

## Notes

Found while building [Build live legendary actions](../closed/057-build-live-legendary-actions.md). 44 legendary actions across the fixtures carry that sentence, 15 of them modeled, mostly on adult and ancient dragons. All have no usage today. The engine now honors CooldownUntilNextTurn on legendary actions, but without the field batch can spend the same action up to three times a round. Change `scripts/import-srd-monsters.py`, regenerate the fixtures and the import review, and add a test that a cooldown legendary action is refused a second time in one round. The fixture fingerprint changes, so land this before the baseline regeneration in the reactions build, and leave `data/baseline/curve.json` alone here. Consult `tdd` and `domain-modeling`.

## Resolution
