---
id: 094
title: Batch value of debuffs
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

Once modifiers land on their target, a homebrew "the target has Disadvantage on attack rolls" rider or Slow's -2 AC lands on an opponent, and policy v3 scores it 0 because ticket 045 valued buffs only. Monsters built around debuffs read weaker in batch than they play. Should a debuff mirror the buff formula (the target's threat or incoming damage it changes), and does that need a new policy version (v4 goes to ticket 099)?

## Notes

- Source: [Haste extra action and ally-targeted buffs](../closed/055-haste-extra-action-and-ally-targeted-buffs.md).
- Related: [093](093-dice-valued-roll-bonuses.md) for Bane-style dice, and [097](../tickets/097-batch-value-of-a-target-teleport.md), which may share the same policy bump.
- A policy change regenerates the baseline curve once. Decide whether 094 and 097 ship in one version after v4. Answered in Resolution item 7: no shared bump.
- Skills: `grilling`, `domain-modeling`. If the answer is code, the build is a follow-up `task` ticket.

## Resolution

Decided in auto mode with no user interview. The user reviews these decisions at the commit gate.

1. **Mirror the buff terms.** A modifier on an opponent is valued with the buff terms mirrored, using the existing constants and no new `policy.*` inputs. AC -n is worth `D × AC_POINT_VALUE × n`, a save penalty of n (flat or a die's mean) `S × SAVE_POINT_VALUE × n`, and Disadvantage on saves `S × SAVE_ADVANTAGE_VALUE`. An attack penalty of n is worth `threat(bearer) × AC_POINT_VALUE × n`. Exact recomputation of threat was rejected as too heavy for a heuristic that already prices a d20 point at 0.1.
2. **Buckets.** Terms that cut the bearer's threat (attack penalties, attack Disadvantage) go in control, as conditions and pushes do. Terms that raise the actor side's damage on the bearer (AC, saves, save Disadvantage) go in damage. Threat reads respect `MAX_THREAT_NESTING`.
3. **Disadvantage on attack rolls** is 5 points: `threat × AC_POINT_VALUE × 5`. Poisoned, whose only combat effect is the same Disadvantage, still scores 0.25 × threat once. That gap goes to fog.
4. **Horizon.** Riders use the buff horizon `h` (cut to the rounds left) and keep `k`, with `f = 0` because the bearer is an opponent, and are scaled by the hit or failure chance. Conditions stay one-shot. Moving conditions onto the horizon would move many baseline cells and force v5, so it goes to fog.
5. **Sides.** Each change counts only on the side it serves: helpful changes on the actor's side, harmful ones on an opponent. The other pairings score 0, never below. For an ability whose effects are all modifiers, `modifier_plan` adds living opponents as candidate bearers for its `target` modifiers, after the actor and allies, only when a harmful change scores above 0, so Haste never picks an opponent. Its score gains the control term.
6. **Unscored targets.** Speed, Reactions, damage-roll and ability-check modifiers on an opponent score 0, as on an ally. Reactions is the first to price later.
7. **Policy version.** It stays v4. No curve input carries a modifier that lands on an opponent and scores. The standard party prepares neither Slow nor Bane, the stone golem's Slow is unmodeled, and Hypnotic Pattern's Speed 0 stays 0. This is the rule of tickets 078 and 093. The build regenerates the curve and stops for v5 if any cell moves. Ticket 097 decides its own version under the same rule. The two don't share a bump.
8. **Build.** Ticket 104, AFK, blocked by 103, which supplies `SAVE_POINT_VALUE` and the die mean Bane needs.
9. **Vocabulary.** No new term. "Debuff" stays an Avoid term under **Modifier**. ADR 0012 says "a modifier on an opponent". `CONTEXT.md` is unchanged.
10. **Same-source guard.** The existing rule (0 on a bearer holding a modifier from the same `source_ability`) applies to opponents unchanged. Riders must go through the same `bearer_value` check so a second Slow on a slowed foe scores 0.
11. **Downed bearers.** A concentration modifier on a downed bearer still blocks a recast, as Haste on a downed ally does now. Changing that changes Haste play on curve inputs, so it goes to fog.
12. **`on_end`.** On an opponent bearer, `on_end` scores 0. The lethargy cost applies only on the actor's side.
13. **Reasons.** A `modifier_plan` on an opponent reads like Haste's reason. Riders fold into the attack or save plan's values without a note, as conditions do.
14. **Whose damage.** D and S come from `incoming_rolls(bearer, …)`, which for an opponent bearer counts the actor's side, the actor included.

ADR 0012 has a new "Modifiers on an opponent" paragraph, and its "debuff values are not decided" sentence is gone.
