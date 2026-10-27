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

Graduated from the fog item "SRD import review" once the Modeled-ability vocabulary closed. "Monster schema and engine state model" fixed the rule that prose wins when the two disagree, and that a monster becomes a fixture only after its review entries are fixed. The vocabulary decides what the structured side of the comparison can express, including `agent_clauses`, inert abilities and the default resolutions the import writes. The import fixes known upstream faults through a correction list, not by editing upstream data, and writes each correction to the review list. Known faults from the SRD ability survey (`docs/research/srd-ability-survey.md` on branch `research/srd-ability-survey`) are the lost recharge on the second breath of five dual-breath dragon families (20 entries) and the missing `damage` entry on 18 flat-1-damage attacks. The Adult Red Dragon Multiattack mismatch from ticket 005 is a third example. Consult `grilling` and `domain-modeling`.

## Resolution
