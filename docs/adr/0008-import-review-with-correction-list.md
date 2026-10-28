# The SRD import fixes upstream faults through a checked-in correction list and gates fixtures on review

The import reviews every monster it converts and writes an import review list on each run. Entries come in four classes. (a) A structured field contradicts the stat block's own prose; prose wins, per the monster schema decision. (b) A structured field is missing that the prose implies, such as the lost recharge on the second breath of dual-breath dragons. (c) The stat block contradicts a ruleset table, such as the Archmage printing CR 12 with XP 8,000 while the SRD p. 256 table gives 8,400; the table wins. The prose-wins rule covers class (a) only. (d) A classification note recording where each ability landed (modeled, partly modeled, inert, unmodeled) and its default resolution, plus spell ids missing from the engine's spell effects file.

Only classes (a) through (c) block. A stat block becomes a fixture when it has zero open mismatches. Fixtures graduate one monster at a time, the engine only ever sees fixtures, and the goal is all 341 entries. Classification notes are informational and never block.

The importer never corrects anything on its own. Prose parsing exists only to detect mismatches, never to produce data, and an ambiguous parse produces no entry. Every fix comes from one of two human-written sources: a correction in `data/srd/corrections.json`, or an entry in the hand-authored effects layer for mechanics the text carries. A correction names the monster id, the dotted field path (the same dotted paths the override commands use), the corrected value, a reason, a citation, and the upstream commit it was written against. The import applies each correction verbatim and writes every application to the review list. After an upstream data bump, a correction that no longer matches anything fails the import outright.

The review list is a generated `data/srd/import-review.md` with a JSON twin, regenerated on every run and committed, so an upstream bump shows its effect as a git diff. The import script prints a summary and exits non-zero while any blocking entry is open.

## Considered options

- Derive values from confident prose parses and apply them automatically. Rejected because deviations from upstream would then live in two places, only one of them human-readable. One checked-in correction file is the single auditable record.
- Treat classification notes as blocking. Rejected because they have nothing to fix; the default resolutions from the effect vocabulary ADR already give unmodeled and inert abilities engine behavior.
- Extend prose-wins to class (c). Rejected because the table is the general rule the engine uses for XP budgets, and the Archmage's 8,000 disagrees with the 2014 printing of the same stat block, which also gave 8,400.

## Consequences

- A fresh upstream bump can fail the import on stale corrections until a human prunes the file. This is deliberate.
- The import review checks only that spell ids a monster references exist upstream. Authoring and checking the spell effects file is its own ticket.
- Where fixtures ship and how releases carry them belongs to the packaging ticket, not this one.
