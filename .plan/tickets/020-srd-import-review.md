---
id: 020
title: SRD import review
type: grilling
mode: HITL
status: open
blocked_by: [013]
claimed: false
---

## Question

How does the import produce the list of mismatches between upstream structured fields and the prose, and how is that list worked through before an SRD monster counts as a fixture? What does the importer correct on its own, and what does it send to the user?

## Notes

Graduated from the fog item "SRD import review" once the Modeled-ability vocabulary closed. "Monster schema and engine state model" fixed the rule that prose wins when the two disagree, and that a monster becomes a fixture only after its review entries are fixed. The vocabulary decides what the structured side of the comparison can express, including `agent_clauses`, inert abilities and the default resolutions the import writes. The import fixes known upstream faults through a correction list, not by editing upstream data, and writes each correction to the review list. Known faults from the SRD ability survey (`docs/research/srd-ability-survey.md`) are the lost recharge on the second breath of five dual-breath dragon families (20 entries) and the missing `damage` entry on 18 flat-1-damage attacks. The CR research (`docs/research/009-cr-and-encounter-guidance.md`, local data check) found one more: the Archmage stat block says CR 12 with XP 8,000 while the SRD table on p. 256 says CR 12 is XP 8,400. The Adult Red Dragon Multiattack mismatch from ticket 005 is a fourth example. Consult `grilling` and `domain-modeling`.

## Resolution
