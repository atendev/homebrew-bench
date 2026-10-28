---
id: 014
title: Standard party build
type: grilling
mode: HITL
status: closed
blocked_by: [009]
claimed: false
---

## Question

How is the standard party for monster tests built: party size, classes, the levels covered, gear and magic items? Classes, spells and items are not modeled systems in phase 1, so how are party members defined as plain stat blocks, and can the user supply their own party?

## Notes

PCs share the stat block schema with optional PC fields. How party members choose actions in batch belongs to Batch tactics policy. Use the CR and encounter guidance findings. Consult `grilling` and `domain-modeling`.

## Resolution

Resolved 2026-10-27 by grilling. See `docs/adr/0010-standard-party-of-four-generated-archetypes.md`.

- The party is four stat blocks named by role, matching the SRD's four-character CR calibration: defender (front line), striker (ranged-capable), healer, arcane caster. Each notes the class it approximates in a comment.
- The caster genuinely casts: 3 to 5 leveled spells per tier across single-target damage, area damage, save-based control, and one buff, plus one at-will damage option. The healer carries 2 to 3 healing spells plus one at-will option. No summons, no zones. This is the phase-1 minimum scope for ticket 023.
- All levels 1 to 20. One designer-set progression table per archetype lives under `data/party/`, a generator in `scripts/` emits the per-level stat blocks, and both tables and generated blocks are committed. The numbers are our own calibration content, not transcribed class progressions; ticket 015 verifies the party empirically against SRD monsters.
- No magic items at any level. Gear exists only as numbers inside the stat block, and the report names the assumption. Revisit if the CR baseline shows the party falling off SRD monsters at high levels.
- PCs at 0 hit points follow the ruleset death-save rules: unconscious, rolling death saves, healable, killable. Monsters die at 0 unless the agent judges otherwise. Dying is a PC-only behavioral field.
- Users can supply their own party: same stat block schema, files in `~/.bench/parties/<name>/`, with `./.bench/parties/<name>/` winning on a same-named party, selected by `bench batch --party <name>`. Unmodeled-ability rules match monsters: batch refuses unresolved unmodeled abilities, live mode labels them agent judgment. The standard party ships built in and is the default; live mode uses the user's party.
