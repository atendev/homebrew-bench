---
id: 039
title: Recharge ranges and recharge after rest
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Parse recharge dice 'N-6' or 'N' for N in 2..=6 in `roll_recharge`, reject unparseable dice at load time, relax the importer's recharge checks, and map recharge-after-rest to `PerRest { rest: short, uses: 1 }`.

## Notes

Consult `implement`, `tdd` and `domain-modeling`. Decision recorded in [closed/035](../closed/035-srd-importer-vocabulary-gaps.md). Unblocks monsters in the ticket 028 importer; rerun `python3 -I scripts/import-srd-monsters.py` and `cargo test -p bench-core` afterward and report the new graduated count.

## Resolution

`Usage::Recharge.dice` accepts "N-6" or "N" for N in 2 to 6. `effect::recharge_minimum` parses it, and a `deserialize_with` check rejects anything else when a stat block loads. `roll_recharge` now rolls against the parsed minimum and errors on unparseable dice instead of silently never recharging. The importer takes recharge 2-6 (written as "N-6", or "6" for 6) and maps recharge after rest (Cloaker) to `PerRest { rest: short, uses: 1 }`, and its prose cross-check compares the minimum roll.

Also fixed: `roll_recharge` keyed ungrouped abilities by name while `resolve_action` keys them by id, so an ungrouped recharge never restored when name and id differed. Both now use the id.

Result: `cargo test` passes (49 bench-core tests, six new) and the importer graduates 305 of 341 monsters, up from 299 (air-elemental, basilisk, blink-dog, cloaker, pit-fiend, water-elemental). Audit: APPROVE. Engine note: nothing resets `PerRest` yet, so recharge after rest is one use per encounter, as ticket 035 intended. The generated JSON schema does not carry a pattern for the dice; only the serde path rejects bad values.
