---
id: 098
title: Stale concentration in old saves
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

A pre-0.1.18 save can hold a down or incapacitated creature with a stale concentration, and an override aimed at it with the same value leaves it. Should loading, or any override, sweep it, given the replay-divergence risk the 082 audit raised?

## Notes

- Source: [Override ends concentration for end state and concentration overrides](../closed/082-override-ends-concentration-for-end-state-and-concentration-overrides.md). The removal half was built in [Removal ends concentration](../closed/083-removal-ends-concentration.md).
- Replay starts from the saved initial state, so a sweep on load could change what old recorded encounters replay to. Check whether any committed save or fixture holds a stale concentration.
- Skills: `grilling`, `domain-modeling`. If the answer is code, the build is a follow-up `task` ticket.

## Resolution

No sweep, on load or on any override, and no code. The live-mode reference now gives the repair a DM already has. Decided in auto mode by a grilling subagent, with no user interview.

The range in the question is too narrow. Saves from before 0.1.19 can hold a stale concentration. From 0.1.10 only overrides made one (conditions until 0.1.18, end state or concentration until 0.1.19). Before 0.1.10 ordinary play made one whenever a concentrating creature dropped, died or gained an incapacitating condition, and that includes the public v0.1.0, the only published release. No path that creates one is known since 0.1.19. No committed fixture holds one: the only concentrator, in the 0.1.5 and 0.1.6 states, is an active caster with no conditions.

- Loading doesn't sweep. `store::load` requires the last log entry's hash to equal the state's game hash, and `replay` compares the final folded hash, so an unrecorded sweep reads as tampering at the next load and as divergence in replay. A recorded sweep would need a new command that fires on read-only loads, kept forever for saves that can only get rarer.
- Overrides don't sweep. The 082 rule stays. A global scan changes the refold of every old override in a history with a stale creature, so replay diverges there and `undo` silently changes the state.
- A same-value override stays a no-op. It changes no data, and aiming the rule at the `concentration` path needs parsing of the `combatants.<id>` and index forms for a case nobody hits on purpose.
- `--clear` stays as documented. Probes on 0.1.25 showed `<id>.concentration --clear` empties the field and leaves what the creature held, on stale and able creatures alike. The live-mode reference has called that the DM's job since 0.1.18, and CHANGELOG 0.1.20 treats a cleared field with held effects as a known state. Changing it reverses a deliberate rule or carves out a case for saves that can only get rarer.
- The repair needs no build. The same probes showed an override to any value other than the one held ends a stale creature's concentration and what it held, and `bench remove` ends both when the creature should leave. The live-mode reference now says so, with the corrected range. The repair works only on a creature that can't concentrate, because a different value on an able creature leaves its held effects.
- No CHANGELOG entry, because no behaviour changes and the Replay compatibility lines stay accurate.
- No `CONTEXT.md` term, because this is a legacy-save artifact and not a domain concept. No ADR, because the decision is easy to reverse.

`undo` already re-derives a pre-0.1.10 drop under current rules and ends the concentration there. Batch mode never loads saves, so batch numbers and the curve are untouched.
