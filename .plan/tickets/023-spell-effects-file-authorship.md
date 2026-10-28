---
id: 023
title: Spell effects file authorship
type: grilling
mode: HITL
status: open
blocked_by: []
claimed: false
---

## Question

How is the engine-owned spell effects file produced and checked? Monster stat blocks reference 116 distinct spells through structured spellcasting blocks and 23 more by name. How much of that set is hand-modeled first, in what order, and what check keeps the file honest against upstream spell text?

## Notes

ADR 0005 routes spells through an effects file keyed by SRD spell id; a spell id absent from the file makes the cast an unmodeled ability with a default resolution. The import review (closed ticket 020) records missing ids as classification notes and checks only that referenced ids exist upstream, so this ticket owns the file's authorship process. The survey (`docs/research/srd-ability-survey.md`) lists which monsters carry spellcasting blocks. The future spells map may replace the file, but phase 1 needs enough of it that spellcasting monsters do not degrade entirely to default resolutions. Consult `grilling` and `domain-modeling`.

## Resolution
