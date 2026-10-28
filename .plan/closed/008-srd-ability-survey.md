---
id: 008
title: SRD ability survey
type: research
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Across the monsters in `data/srd/` (SRD 5.2), which ability patterns recur and how often? Which patterns are mechanical (rolls, saves, conditions, damage, recharge, movement, area shapes) and which are prose-only? Which abilities fit no pattern?

## Notes

Local data only. Use the `research` skill and write the findings as a Markdown file with a pattern list, a count per pattern, example monsters per pattern, and a list of abilities that fit nothing. This feeds Modeled-ability vocabulary. Do not launch the research agent until the current planning work is committed, so the throwaway `research/srd-ability-survey` branch starts from a clean tree.

## Resolution

Resolved by a research agent on 2026-10-27. Findings live in `docs/research/srd-ability-survey.md`, reproducible with `python3 -I scripts/survey-srd-abilities.py`.

341 monster entries hold 1,392 ability entries under 422 distinct names. Six pattern families (attack rolls, saves with branches, areas, usage limits, multiattack, spellcasting-by-reference) plus condition and movement effects cover almost everything: 83 entries (6 percent) land in the leave-unmodeled bucket, and only 2 match no pattern at all. The findings confirm the modeled line drawn in Modeled-ability vocabulary, and add the suggested cut if the vocabulary ever shrinks. Two upstream data faults are confirmed for the import correction list: 20 dual-breath dragon second-breath entries lost their `usage` field, and 18 flat-1-damage attacks carry no `damage` entry.

Surfaced: the two data faults are already wired into SRD import review. Nothing else new.
