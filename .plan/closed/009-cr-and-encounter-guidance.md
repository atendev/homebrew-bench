---
id: 009
title: CR and encounter guidance in SRD 5.2
type: research
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

What CR guidance does SRD 5.2 itself provide (expected damage per round, HP, AC, attack bonus and save DC by CR, XP thresholds, encounter budgets)? What exists only in the 2024 DMG? What license terms limit shipping or quoting those numbers in this repo?

## Notes

Primary sources only: the SRD 5.2 text and its license. Use the `research` skill and write the findings as a Markdown file that cites page or section for every number. This feeds Standard party build and CR baseline. Do not launch the research agent until the current planning work is committed.

## Resolution

Resolved by a research agent on 2026-10-27. Findings live in `docs/research/009-cr-and-encounter-guidance.md`, every number cited to an SRD 5.2.1 page.

SRD 5.2.1 gives exactly three CR-dependent tables: XP by CR, proficiency bonus by CR, and XP budget per character at Low, Moderate and High difficulty for levels 1 to 20, plus prose calibration (one monster of CR equal to party level is Low for four characters) and running-a-monster advice that sets the assumed tactics for CR. It has no per-CR HP, AC, attack bonus, damage-per-round or save-DC table, and no adventuring-day budget. Whether the 2024 DMG's "Creating a Creature" chapter holds one is unconfirmed because the DMG is paywalled; the question stays open for the user to check a copy. The license is CC-BY-4.0: numbers may ship with verbatim attribution from SRD p. 1, modification must be indicated, trademarks are not licensed, and non-SRD monsters must not be added. The current `data/srd/README.md` attribution uses outdated wording and the old URL.

Surfaced: a new ticket, License and attribution notice, for the attribution fix. The CR baseline ticket inherits the options the findings list (XP-only comparison, empirical curve from SRD stat blocks, or user-supplied DMG numbers), and the Archmage XP mismatch (8,000 in its stat block versus 8,400 in the table) is a fourth known SRD inconsistency for the import review list. Unblocks Standard party build.
