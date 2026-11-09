---
id: 111
title: Build batch Counterspell and the Protective Magic split
type: task
mode: AFK
status: closed
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

Built in auto mode on 2026-11-08. Engine 0.1.30, policy v5.

- **Counter rule.** The policy counters when the cast's value beats the reaction reserve plus the counter's resource cost, and it logs the reason. Value is damage capped at the target's hit points, healing capped at missing hit points, control as scored, and a heal on a downed target priced as a revival. Buffs score zero, so they are never countered. The spell-cast window now carries the cast's targets.
- **Usage.** `per_day` takes an optional `group`, read and spent through one `usage_key`.
- **Protective Magic.** The importer splits it into `protective-magic-shield` and `protective-magic-counterspell`, on one 3/day pool for the Mage and Archmage and with no limit for the Lich. The Counterspell effects entry has no agent clauses left.
- **DC.** The three fixtures had no `spellcasting` block, so the counter's DC stayed 0 and no counter could land. The importer now writes one from the Spellcasting action, for these three only.
- **Curve.** Only these three moved. Mage: 15 cells, party wins 66 to 78 of 100 at level 3, so it got easier at levels 2 to 4. Archmage: 20 cells, at most 6 wins. Lich: 19 cells, party wins 74 to 52 at level 18.

The decision report is [111-implementation](../reports/111-implementation.md).
