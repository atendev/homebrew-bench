---
id: 020
title: SRD import review
type: grilling
mode: HITL
status: closed
blocked_by: [013]
claimed: false
---

## Question

How does the import produce the list of mismatches between upstream structured fields and the prose, and how is that list worked through before an SRD monster counts as a fixture? What does the importer correct on its own, and what does it send to the user?

## Notes

Graduated from the fog item "SRD import review" once the Modeled-ability vocabulary closed. "Monster schema and engine state model" fixed the rule that prose wins when the two disagree, and that a monster becomes a fixture only after its review entries are fixed. The vocabulary decides what the structured side of the comparison can express, including `agent_clauses`, inert abilities and the default resolutions the import writes. The import fixes known upstream faults through a correction list, not by editing upstream data, and writes each correction to the review list. Known faults from the SRD ability survey (`docs/research/srd-ability-survey.md`) are the lost recharge on the second breath of five dual-breath dragon families (20 entries) and the missing `damage` entry on 18 flat-1-damage attacks. The CR research (`docs/research/009-cr-and-encounter-guidance.md`, local data check) found one more: the Archmage stat block says CR 12 with XP 8,000 while the SRD table on p. 256 says CR 12 is XP 8,400. The Adult Red Dragon Multiattack mismatch from ticket 005 is a fourth example. Consult `grilling` and `domain-modeling`.

## Resolution

The import produces the review list from four check classes. (a) A structured field against the stat block's own prose, where prose wins. (b) A structured field missing that the prose implies. (c) The stat block against a ruleset table, where the table wins; the Archmage is corrected to XP 8,400, citing SRD p. 256 against p. 305. (d) Informational classification notes recording where each ability landed and its default resolution, plus spell ids missing from the spell effects file and an integrity check on referenced ids. Only (a) through (c) block. A monster becomes a fixture when its mismatches reach zero, fixtures graduate one at a time toward all 341, and the engine sees fixtures only.

Fixes come from exactly two human-written sources. `data/srd/corrections.json` holds one entry per upstream fault (monster id, dotted field path from the override vocabulary, corrected value, reason, citation, upstream commit), applied verbatim and logged to the review list. The hand-authored effects layer carries mechanics that live only in text. The importer auto-corrects nothing: prose parses detect mismatches but never become data, ambiguous parses are silent, and a stale correction after an upstream bump fails the import.

The list is a generated, committed `data/srd/import-review.md` with a JSON twin. The import script exits non-zero while blocking entries are open, and a human works the list. This decision created two follow-ups: ticket 022 (seed corrections for the four known faults) and ticket 023 (spell effects file authorship). Fixture shipping stays with the packaging ticket. See `docs/adr/0008-import-review-with-correction-list.md`.
