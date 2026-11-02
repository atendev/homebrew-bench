---
id: 050
title: Upcast spell scaling
type: grilling
mode: HITL
status: open
blocked_by: []
claimed: false
---

## Question

How does the spell effects file state what changes per slot level above a spell's base level, and how does `resolve_cast` apply it? Decide the field shape (extra damage dice, extra targets, longer duration), whether scaling is scaffolded from the SRD's higher-level text by `scripts/scaffold-spells.py`, and how a cast with no scaling entry is reported.

## Notes

`resolve_cast` ignores `Cast.slot_level` and `upcast`, so a dragon's level 3 Acid Arrow resolves at the base level. ADR 0014 governs the effects file and its honesty checks. Cantrip scaling by character level may share the answer. Consult `grilling` and `domain-modeling`.

## Resolution
