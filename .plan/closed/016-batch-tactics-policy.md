---
id: 016
title: Batch tactics policy
type: grilling
mode: HITL
status: closed
blocked_by: [013, 014]
claimed: false
---

## Question

What built-in policy picks actions for both sides in `bench batch`? Cover target selection, action choice, use of recharge and limited-use abilities, legendary actions, movement between range bands, the default area target count by area size, and when party members heal or use consumables.

## Notes

Batch is one engine call, so the agent does not choose per-turn actions. The step-engine ADR (`docs/adr/0003-step-engine-with-abstract-engagement.md`) fixed the band movement costs and left the policy open. A standing assumption can override the policy's defaults. Consult `grilling` and `domain-modeling`.

## Resolution

Resolved 2026-10-28 by grilling. See `docs/adr/0012-deterministic-batch-tactics-policy.md`.

- Batch uses one deterministic, versioned policy for monsters and PCs. Standing assumptions can override named policy constants and ordering rules.
- Legal move-plus-action plans are scored together using expected damage, control and survival weights of 1.0, with resource cost at 0.0 for the single encounter. Special and limited-use abilities are used whenever they score higher than the best at-will action.
- Target priority includes the next opponent action cycle's damage, control, healing and limited-use threat; ties use immediate damage, lowest HP percentage and stable combatant ID. Downed targets are excluded unless explicitly enabled.
- Recharge abilities require a successful rules-based recharge. Legendary actions are used in their current eligible window and never carried into a later round.
- Movement follows the established range-band costs and occurs only when it enables or improves an action. Default area target counts are 1, 2, 3, and 4/all for single, small, medium, and large or battlefield-wide areas; ally harm and wasted damage count against the score.
- The healer revives when revival is legal and useful, heals living allies below 50% only when they are likely to be downed before their next turn, and non-healers use consumables below 25% or under lethal threat. Modeled variable effects use expected value; unresolved effects are unavailable.
- Every decision logs its reason and relevant assumptions. Invalidated plans are recomputed; no legal plan falls back to the standard defensive/no-op action. Policy version and constants enter baseline fingerprints.
