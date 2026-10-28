---
id: 024
title: Standard party spell list
type: prototype
mode: HITL
status: closed
blocked_by:
  - 023
claimed: false
---

## Question

Which exact spells satisfy the standard party minimums decided in ticket 014, now that ticket 023 has fixed the effects file location, schema, and authorship process? Produce a concrete, committed list for the caster and healer archetypes that the standard party generator can reference.

## Notes

The standard party build decision (closed ticket 014) requires:
- Caster: 3 to 5 leveled spells per tier covering single-target damage, area damage, save-based control, and one buff, plus one at-will damage option; no summons, no zones.
- Healer: 2 to 3 healing spells plus one at-will option.

Ticket 023 settled the file path (`data/spells/effects.json`), the schema (`id`, `name`, `source`, `effects`, optional `notes`), and the growth rule. This ticket does not need to model every referenced spell; it only needs the party's list. Each chosen spell must already exist or be added to the SRD 2024 spells file, and each needs an effect entry authored per the schema.

The artifact should be a small file or snippet in the proposed standard-party generator location, showing the caster and healer spell choices by tier, with SRD spell ids and a note about which already have effects-file entries and which still need them.

## Resolution

Resolved 2026-10-28. The committed artifact is `data/party/standard-spells.json`.

- The caster has one at-will damage spell and four leveled spells in each tier from 1
  through 9, covering single-target damage, area damage, save-based control, and a
  defensive or utility buff. No summons or zones are included.
- The healer has one at-will option and two or three leveled healing or recovery
  spells in each tier where the 2024 SRD provides them. Tier 8 is intentionally
  empty because the SRD has no direct healing or recovery spell at that level.
- Every entry includes its SRD id, display name, category, and source. The snapshot
  contains every selected id. The effects file does not exist yet, so the artifact
  marks every selection as `needs-effects-entry`; this is the honest handoff for
  the effects-file authoring work described in ticket 023.
