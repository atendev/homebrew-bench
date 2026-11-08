---
id: 100
title: Build darkness
type: task
mode: AFK
status: closed
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

Built on branch `plan/100-build-darkness`. Engine 0.1.26, `POLICY_VERSION` stays 4.

- `Lighting::Darkness` (additive at schema version 1). `meets`: `darkness` meets a `darkness` or `dim_light` key; `dim_light` doesn't meet `darkness`. `--lighting darkness` and the override path take the snake_case spelling.
- `unseen_reason` checks Invisible, then Blinded, then darkness, so each event names one reason: `<observer> cannot see <observed> in darkness`. Tests cover attacks both ways (a Darkvision monster against the party; two non-seers roll straight), Opportunity Attack eligibility and its refusal text, and a ranged attack in close combat. Frightened reads the same `sees`.
- Batch: the Lighting row states darkness as the ticket words it. When lighting isn't `darkness` and sight is uneven (or a trait is keyed to `darkness` or `dim_light`), a warning says darkness would change the result and gives the first sample's replay command with `--lighting darkness`.
- Proof: a regenerated curve differs from the committed one only on `engine_version`, so numbers under `unspecified` are unchanged. The embedded-curve CLI test fails on a stale engine version, so the version line in `data/baseline/curve.json` was edited to 0.1.26, as in commit 4502693.
- Docs: CHANGELOG line, ADR 0005 (a `dim_light` key now applies under `darkness`), and the live-mode and batch-mode references. In `tactics.rs` nothing else scores sight.
- Decision report: [100-implementation](../reports/100-implementation.md).
