---
id: 086
title: Adding a combatant by override
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

[ADR 0006](../../docs/adr/0006-override-paths-and-log-format.md) says adding a combatant is not an override and `bench add` is the way. The 085 audit read `override_path.rs` and `live.rs` and found that setting a `combatants` list with a new id is not refused: the creature joins with no initiative and no ADD line. Should the override path refuse an added id and point at `bench add`, or accept it and roll initiative and log it?

## Notes

- Reproduced on engine 0.1.21: `bench start goblin-warrior party=goblin-boss`, then `bench override combatants <list plus a copy with id goblin-warrior-9>`. The call succeeds. The new creature is in `combatants` but not in `initiative_order` or `initiative_rolls`, the log has one OVERRIDE line and no ADD line, and the opponent has no range entry back to it. Four `end-turn` calls cycle goblin-warrior and goblin-boss only, so it never acts. `replay` still matches. A creature that never acts but stays in `combatants` is also a live body for any win check.
- ADR 0006 rejected "Adding combatants through `bench override`" because an add rolls initiative and picks a starting band, which deserves its own log tag. Refusing matches that. Accepting would reverse it and need a new ADR.
- Check whether the refusal should cover a clear-then-set of the same list, and whether a pending prompt or the active-turn creature changes the rule, as they do for removal ([ADR 0006 and override removal](../closed/085-adr-0006-and-override-removal.md)).
- Replay risk: a refusal changes how old logs replay only if an old save already added by override.
- Skills: `grilling`, `domain-modeling`. If the answer is code, the build is a follow-up `task` ticket.

## Resolution

Refuse. An override that leaves `combatants` holding an id the encounter did not have fails whole, with an error that names `bench add`. This matches [ADR 0006](../../docs/adr/0006-override-paths-and-log-format.md), which already rejects adding combatants through override because an add rolls initiative and picks a starting band, and the override has no place to take either. Accepting would reverse the ADR and need a new one. The refusal covers the list form and an index past the end. It is the same shape as the removal refusals: nothing is applied and no log line is written. Build is [ticket 087](../tickets/087-refuse-added-ids-in-override.md).
