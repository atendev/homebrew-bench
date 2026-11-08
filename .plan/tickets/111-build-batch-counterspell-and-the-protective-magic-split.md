---
id: 111
title: Build batch Counterspell and the Protective Magic split
type: task
mode: AFK
status: open
blocked_by: [110, 102]
claimed: false
---

## Question

Build the policy and data half of [Counterspell and the spell-cast trigger](../closed/101-counterspell-and-the-spell-cast-trigger.md): the batch counter rule, the `group` key on `per_day` usage, the Protective Magic split, the fixture re-import, and the regenerated baseline.

## Notes

- Skills: `implement`, `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- Design: [ADR 0019](../../docs/adr/0019-counterspell-through-a-spell-cast-window.md). It splits the Protective Magic entry that 102 leaves whole with its Shield cast.
- Policy: counter when the reaction is free and the cast's expected value beats the reaction reserve plus the slot or usage cost. Deterministic, logged reason, never for a zero-value spell. Bump `POLICY_VERSION`.
- `per_day` usage gains an optional `group`; entries sharing one share a pool. Protective Magic on the Mage and Archmage (3/day) shares across both entries, the Lich has no limit. Re-import the three fixtures and the Counterspell effects file entry (`counter_spell`, no agent clauses left).
- The standard party gets no Counterspell.
- Regenerate the baseline curve and fingerprints, and report how far the Mage, Archmage and Lich move.

## Resolution

<Filled in at close.>
