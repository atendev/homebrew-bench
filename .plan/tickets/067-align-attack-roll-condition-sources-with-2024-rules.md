---
id: 067
title: Align attack-roll condition sources with the 2024 conditions
type: grilling
mode: HITL
status: open
blocked_by: []
claimed: true
---

## Question

Which of the gaps between the engine's attack-roll Advantage and Disadvantage sources and the 2024 conditions should the engine close, how should the position-dependent ones read without a grid, and should the fix land before ticket 056 regenerates the baseline?

## Notes

Found while building [Fix Advantage and Disadvantage cancellation](../closed/061-fix-advantage-disadvantage-cancellation.md). `disposition` in `bench-core/src/engine.rs` feeds `combined_disposition`, so a new source is one more entry in its list.

Clear-cut gaps (PHB Appendix C):

- Blinded target: attack rolls against it have Advantage (p. 360).
- Restrained attacker: its attack rolls have Disadvantage (p. 372).
- Petrified target: attack rolls against it have Advantage (p. 371).

Gaps that need a reading without positions:

- Grappled attacker: Disadvantage against any target other than the grappler (p. 366). `ConditionInstance.source` could name the grappler.
- Prone target: Advantage only when the attacker is within 5 feet, else Disadvantage (p. 371). The engine always gives Advantage. `engaged` could stand for within 5 feet.
- Frightened attacker: Disadvantage only while the source is in line of sight. The engine always applies it.
- Invisible: the engine ignores Blindsight, Truesight and See Invisibility on the other side.

Any fix changes batch numbers and moves the engine one patch. If it should share ticket 056's regeneration, 056 needs this ticket in `blocked_by`. The tactics policy's own `disposition` in `tactics.rs` has the same list, and 056 rewrites it under v3.

## Resolution
