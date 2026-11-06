---
id: 100
title: Build darkness
type: task
mode: AFK
status: open
blocked_by: [091]
claimed: false
---

## Question

Build the decision in [Darkness and Darkvision](../closed/091-darkness-and-darkvision.md): the `darkness` lighting value, the darkness reason in the sight rule, the `dim_light` key met by darkness, the report's Lighting row and uneven-sight hint, and the docs.

## Notes

- Skills: `implement`, `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- `Lighting` (`bench-core/src/effect.rs` ~132): add `Darkness`, an additive variant at schema version 1. `meets`: `darkness` meets a `darkness` or `dim_light` key, and `dim_light` doesn't meet `darkness`. Check that the CLI flags and the override path take the new snake_case spelling.
- Sight (`engine.rs` `unseen_reason` ~2443): under `darkness`, an observer with no Darkvision, Blindsight or Truesight sees no one. The reason reads "<observer> cannot see <observed> in darkness". Check the order against the Invisible and Blinded reasons so each event names one reason. Test attacks both ways (Darkvision monster against the party, two non-seers rolling straight), Opportunity Attack eligibility (~1466) and its refusal text naming the darkness reason, Frightened, and ranged attacks in close combat.
- Tactics: the policy reads darkness only through `disposition_with` and `can_act`. Confirm nothing else in `tactics.rs` scores sight on its own.
- Batch report (`batch.rs` `lighting_line`, `light_words`, Standing assumptions):
  - Lighting row under darkness: "darkness (nonmagical), a creature without Darkvision, Blindsight or Truesight sees no one; spell targeting not gated by sight".
  - When the lighting isn't `darkness` and the sight is uneven (the combatants that fought, monster and party, don't all have or all lack Darkvision, Blindsight or Truesight, or any of them has a roll modifier keyed to `darkness` or `dim_light`), add a hint saying darkness would change the result and giving the replay command with `--lighting darkness`.
- Live mode: the digest carries `lighting` already and gets no per-combatant sight field. The live-mode reference in the skill says what darkness does, that a party with torches fights in `dim_light` or `bright_light`, that a trait working only in Dim Light must not be keyed `dim_light` (darkness meets that key), and that magical Darkness and sight-gated targeting stay the agent's call.
- Docs: ADR 0017 and `CONTEXT.md` were updated in ticket 091. Add a CHANGELOG line.
- Versions: bump the workspace one patch. `POLICY_VERSION` stays 4 and the curve is not regenerated. Prove that batch numbers under `unspecified` are unchanged on the fixtures. If any change, stop and report.

## Resolution

<Filled in at close.>
