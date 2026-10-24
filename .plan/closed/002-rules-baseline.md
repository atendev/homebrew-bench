---
id: 002
title: Rules baseline
type: grilling
mode: HITL
status: closed
blocked_by: [001]
claimed: false
---

## Question

Which edition and ruleset does the engine model, and what data source does it load from?

## Notes

`data/srd/` already holds SRD 5.1 and 5.2 JSON from an earlier pass. Treat that as available material, not a choice.

## Resolution

Decided with the user on 2026-10-24.

- The engine models the 2024 rules only, as published in SRD 5.2. One edition in phase 1. The reason is that 2024 is the most current reading of the rules.
- The boundary is the combat rules in SRD 5.2, implemented as written. Optional rules and anything outside SRD 5.2 are not modeled by default.
- SRD monsters serve three purposes: test fixtures for the engine, a reference set for comparing homebrew results against real monsters, and a library of monsters to fight in live mode. How the comparison works belongs to the CR baseline.
- Every monster, SRD or homebrew, passes through an import step into our own format. The engine reads only that format, never the upstream 5e-database JSON. The format itself is the monster schema ticket.
- At intake, the agent converts 2014-style stat blocks to the 2024 format and flags what it had to guess. The engine never sees the older layout.
- `data/srd/2014/` and its SRD 5.1 attribution come out of the working tree. The import script can still fetch it. Removal is follow-up work, not part of this ticket.
- **House rules.** Users can add rules beyond SRD 5.2, either to fill a gap or to change an SRD rule. In phase 1 the engine knows nothing about them. The agent reads them as prose and applies them through overrides, and the log names the house rule as the reason.
- House rules live as local files, one per rule, in a gitignored folder at the repo root. They follow a template with fixed sections so they are easy to navigate. The agent writes a file only after the user confirms the wording, and reads the folder at the start of every session. The template and folder name are for the skill ticket.
- The skill has the agent state the rules boundary and list the loaded house rules at the start of a run. When a situation is not covered, the agent says so, labels its call as agent judgment, and offers to save a house rule. Batch reports list the house rules and agent judgments that affected the numbers.
- Later direction, outside phase 1: the agent could turn a house rule into engine code. The house rule format should leave room for an engine-modeled flag.
