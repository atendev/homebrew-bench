---
id: 022
title: Seed corrections for known upstream faults
type: task
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

Write the corrections and effects-layer entries for the four known upstream faults and drive the import until all 341 stat blocks are fixtures. Record the final state of the import review list.

## Notes

The four known faults: the lost recharge on the second breath of five dual-breath dragon families (20 entries), the missing damage entry on 18 flat-1 attacks, the Archmage CR 12 XP (corrected to 8,400 against the stat block's 8,000), and the Adult Red Dragon Multiattack count (three Rends, not two). The mechanics live in closed ticket 020 and `docs/adr/0008-import-review-with-correction-list.md`. This task needs the importer's conversion and review pipeline to exist, and it is that pipeline's first real run. The resolution records the final counts: corrections applied, mismatches closed, classification notes outstanding.

## Resolution

Resolved with the user on 2026-10-28. Audited all 341 monsters in `data/srd/2024/5e-SRD-Monsters.json` (`a6212beb4b278917c2cff41c2b04c4ef4f3e0d6f`) against the check classes in `docs/adr/0008-import-review-with-correction-list.md`, committed the seed `data/srd/corrections.json` (72 field-path corrections across 50 monsters), updated `docs/adr/0005-closed-effect-vocabulary-with-agent-clauses.md` and `docs/adr/0008-import-review-with-correction-list.md`, and left building the converter/effects layer and generating `data/srd/import-review.md` to implementation.

- **Dual-breath metallic dragons (20 monsters, 40 entries).** All 20 stat blocks across the five metallic dragon families (`brass`, `bronze`, `copper`, `gold`, `silver` × 4 ages) drop `usage` on the second breath (`Sleep Breath`, `Repulsion Breath`, `Slowing Breath`, `Weakening Breath`, `Paralyzing Breath`). In 5e rules, both breaths sit under one `Breath Weapons (Recharge 5–6)` header and share one cooldown and recharge roll. Added an optional `group` key to `recharge` usage in ADR 0005 and wrote 40 entries in `data/srd/corrections.json` setting `usage: {"kind": "recharge", "min": 5, "group": "breath_weapons"}` on both breath actions of each of the 20 dragons.
- **Attacks missing `damage` (18 survey entries split into 16 flat-1 attacks, 1 zero-damage grapple, and 1 spliced monster).**
  - **16 genuine flat-1 damage attacks** (`awakened-shrub` Rake, `badger` Bite, `bat` Bite, `cat` Scratch, `crab` Claw, `frog` Bite, `giant-fire-beetle` Bite, `hawk` Talons, `homunculus` Bite, `lizard` Bite, `owl` Talons, `piranha` Bite, `rat` Bite, `raven` Beak, `sprite` Enchanting Bow, `weasel` Bite): patched in `data/srd/corrections.json` with `[{"amount": 1, "damage_type": "<type>"}]`.
  - **`roper` `Tentacle` (1 entry):** legitimately deals zero damage on hit (inflicting `Grappled` and `Poisoned` only). Refined class (b) in ADR 0008 so missing `damage` on an attack roll is flagged only when the prose `Hit:` clause specifies damage.
  - **`mule` spliced with `octopus` (1 monster, 3 entries):** upstream `mule` splices Mule's top-level stats with Octopus's `special_abilities` (`Compression`, `Water Breathing`), `actions` (`Tentacles`), and `reactions` (`Ink Cloud`), while `octopus` is absent from the file. Added a creature-name self-reference check to class (a) in ADR 0008 (flagging ability prose that names a different creature, such as `"The octopus"` on `mule`) and patched `mule`'s `traits`, `actions`, and `reactions` in `data/srd/corrections.json` with Mule's SRD 5.2.1 p. 352 `Beast of Burden` and `Hooves` (`+4, reach 5 ft., 4 (1d4 + 2) Bludgeoning damage`).
- **Multiattack encoding vs. genuine upstream faults (34 converter-handled swap entries, 12 corrected faults).**
  - Across **34 monsters** whose prose reads *"makes N `<X>` attacks. It can replace K attack(s) with `<Y>`"* (including `adult-red-dragon`, `gladiator`, `dragon-turtle`, `solar`), `5e-database` systematically encodes `(N - K)` unconditional attacks in `actions[]` plus `choose 1 of [K <X>, <Y>]` in `action_options` (and on 20 spell-swap entries puts the spell's display name in `action_options` instead of `Spellcasting`). The converter folds `(N - K) + choose(K, swap)` into `{action: X, count: N}` with swap `<Y>` and resolves spell display names against the monster's `Spellcasting` block, keeping those 34 out of `corrections.json`.
  - **12 genuine `Multiattack` faults** are patched in `data/srd/corrections.json`: `dryad` (fixing `choose 1 of [Vine Lash, Thorn Burst, Charm Monster]` to `choose 1 of [Vine Lash, Thorn Burst]` plus optional `Charm Monster`), `erinyes` (fixing `action_name: "Entangling Rope"` to match `"Entangling Rope (Requires Magic Rope)"`), and the **10 non-hybrid werebeast form blocks** (`-human` and `-<animal>` for `werebear`, `wereboar`, `wererat`, `weretiger`, `werewolf`, where upstream stripped the other forms' attack actions from the form stat block but left the full multi-form `Multiattack` prose untouched).
- **Ruleset table checks (1 entry).** Confirmed across all 341 stat blocks that `archmage` (`xp: 8000` vs. CR 12 table `8400`, SRD 5.2.1 p. 256) is the only CR-to-XP or CR-to-PB mismatch. Patched in `data/srd/corrections.json`.
