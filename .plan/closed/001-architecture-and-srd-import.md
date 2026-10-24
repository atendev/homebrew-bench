---
id: 001
title: Architecture and SRD import
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

Is a skill enough to test homebrew, and where does the rules data come from?

## Notes

Decided in an earlier chat before this repo existed. Recorded here so the map has a starting point.

## Resolution

A skill alone isn't enough. An agent doing combat math in its head drifts: it forgets concentration, misapplies resistances, and miscounts rounds. So the math lives in code. The agent reads the homebrew, drives the engine, and interprets results.

Pieces:

1. SRD JSON for exact lookups. No vector database at first. Prose rules (grappling, cover, concentration) go in a few markdown files.
2. A homebrew schema close to the SRD shape, plus our own `effects` layer for prose-only mechanics. Anything outside the effects vocabulary is marked unmodeled and reported.
3. A combat engine covering dice, initiative, attacks, saves, damage types, conditions, concentration, action economy, and resources. Not a full 5e implementation.
4. Test scenarios per homebrew type. Monsters fight a standard party. Classes are compared on damage per round and survivability. Items and spells are compared at the same rarity or slot level.
5. A report with numbers, a short readout, and intake wording problems.
6. The skill is the front door. It ships in the same repo as the engine and data.

Limits the tool must state: simulated parties play optimally and never tire, so numbers are guidance. Social, exploration, and narrative features are out of scope. Only SRD content ships in the repo.

Phases: 1 monsters, 2 items and spells, 3 classes and subclasses, 4 prose rules reference.

SRD import: `scripts/import-srd.sh` pulls 5e-bits/5e-database at a pinned commit into `data/srd/2014` (SRD 5.1) and `data/srd/2024` (SRD 5.2), with upstream TypeScript schemas. Attribution is in `data/srd/README.md`.

Data facts at import (commit a6212be):

| | 2014 | 2024 |
|---|---|---|
| Monsters | 334 | 341 |
| Actions | 841 | 838 |
| Multiattacks, all structured | 148 | 188 |
| Other actions with attack, dc, or damage fields | 616 of 693 | 578 of 650 |
| Special abilities, mostly prose | 551 | 354 |
| Bonus actions | 0 | 89 |
