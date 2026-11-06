---
id: 086
title: Adding a combatant by override
type: grilling
mode: HITL
status: open
blocked_by: []
claimed: false
---

## Question

[ADR 0006](../../docs/adr/0006-override-paths-and-log-format.md) says adding a combatant is not an override and `bench add` is the way. The 085 audit read `override_path.rs` and `live.rs` and found that setting a `combatants` list with a new id is not refused: the creature joins with no initiative and no ADD line. Should the override path refuse an added id and point at `bench add`, or accept it and roll initiative and log it?

## Notes

- Not yet reproduced. Reproduce first: override a live encounter's `combatants` with a list holding one new id, then read the log and the turn order.
- ADR 0006 rejected "Adding combatants through `bench override`" because an add rolls initiative and picks a starting band, which deserves its own log tag. Refusing matches that. Accepting would reverse it and need a new ADR.
- Check whether the refusal should cover a clear-then-set of the same list, and whether a pending prompt or the active-turn creature changes the rule, as they do for removal ([ADR 0006 and override removal](../closed/085-adr-0006-and-override-removal.md)).
- Replay risk: a refusal changes how old logs replay only if an old save already added by override.
- Skills: `grilling`, `domain-modeling`. If the answer is code, the build is a follow-up `task` ticket.

## Resolution

<Filled in at close: the decision and the reason for it.>
