---
id: 091
title: Darkness and Darkvision
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

An encounter-wide `darkness` lighting value, read against each creature's Darkvision, Blindsight and Truesight, would need no positions: a creature that can't see in it is Blinded. How does the standard party see in it, how does a homebrew "Dim Light or Darkness" trait map, and does the batch report need a second run in darkness?

## Notes

- Source: [Encounter lighting setting](../closed/049-encounter-lighting-setting.md).
- ADR 0017 treats every sense as reaching every band, so sense range does not count. Position-dependent vision stays out of scope.
- The standard party has no Darkvision and carries no light. Decide whether it gets a light source by default or the setting is simply a lighting value the agent sets.
- Skills: `grilling`, `domain-modeling`. If the answer is code, the build is a follow-up `task` ticket and may need a new ADR.

## Resolution

Decided on 2026-11-06 in auto mode. The main thread ran `grilling` and `domain-modeling`; a grilling subagent answered 14 questions over two rounds and accepted every recommendation, revising one (versions) after a fact correction. The answers are grouped into the bullets below. Evidence: ticket 049, ADR 0010 and 0017, `bench-core/src/effect.rs` (`Lighting::meets`), `engine.rs` (`sees`, `unseen_reason`, `disposition_with`), `stat_block.rs` (`Senses`), `batch.rs` (lighting lines), and the 341 fixtures: 210 have Darkvision, 80 Blindsight, 16 Truesight, and 80 have none of the three.

- **`darkness` is a fifth lighting value.** It is set the same ways as the others: `--lighting darkness` on `bench start` and `bench batch`, and `bench override settings.lighting darkness`. Reason: it needs no positions. Without it, batch can't express darkness, and in live mode the agent has to add and later remove Blinded by hand.
- **Darkness is read through the sight rule, not stored.** Under `darkness`, `sees()` gains a reason: an observer with no Darkvision, Blindsight or Truesight sees no one. Sense ranges still don't count (ADR 0017). Nothing goes into `conditions`. Reason: the 2024 rule makes a creature Blinded "when trying to see something in that area", which is the question `sees()` answers. Storing Blinded, or reading it at roll time, would give Advantage to an attacker that can't see the target either, and a stored condition breaks when the lighting is overridden or undone.
- **It reaches every rule that reads sight**: attack rolls (an unseen attacker has Advantage and an unseen target gives Disadvantage, so two non-seers roll straight), Frightened's source check, ranged attacks in close combat, and Opportunity Attacks. A non-seer makes no Opportunity Attacks in darkness. Frightened's "within line of sight" clause was already read as the sight rule in ADR 0017 (ticket 067), so darkness cuts it too. The tactics policy follows through the shared `disposition_with`. Reason: one sight rule is the point of ADR 0017.
- **Nonmagical only.** Magical Darkness (the Darkness spell, the darkmantle's aura), which blocks Darkvision, stays the agent's call, and the schema doesn't gain the "unimpeded by magical darkness" note. Reason: magical darkness is nearly always a local area, which needs positions.
- **The standard party sees no one in darkness.** The value means nobody has light. A party with torches or the Light cantrip is fighting in `dim_light` or `bright_light`, and the user picks that. Reason: light sources are out of scope without positions (049), the party carries no gear (ADR 0010), and giving it a default light would make `darkness` mean something else.
- **"Dim Light or Darkness" traits are keyed `dim_light`.** Darkness meets a `dim_light` key, as sunlight meets `bright_light`, and a new `darkness` key is met only by `darkness`. A trait that works only in Dim Light must not be keyed `dim_light`, because the engine would apply it in darkness too. It stays unmodeled and the agent's call, and the live-mode reference says so. The shadow's Hide trait stays unmodeled. Reason: "Dim Light or Darkness" is the common 2024 wording, and the key stays a single value.
- **No automatic second batch run.** Darkness is a separate batch the user asks for. When the lighting isn't `darkness` and the sight is uneven (the combatants that fought don't all have or all lack Darkvision, Blindsight or Truesight, or any of them has a roll modifier keyed to `darkness` or `dim_light`), Standing assumptions gains a hint: darkness would change the result, with the replay command using `--lighting darkness`. Reason: a second pass doubles run time, and the curve has no darkness baseline. When nobody sees, attacks mostly cancel, so the hint would be noise.
- **Log and digest.** Attack events and Opportunity Attack refusals name the reason in the existing format, e.g. `(Disadvantage: pc-striker cannot see goblin in darkness)`. The digest already carries `lighting` and gets no per-combatant field.
- **Batch report under darkness.** The Lighting row reads, for example, "Lighting: darkness (nonmagical), a creature without Darkvision, Blindsight or Truesight sees no one; spell targeting not gated by sight". The existing basis line (the curve fought with lighting unspecified) applies.
- **Versions.** The workspace takes one patch, as every build does. `POLICY_VERSION` stays 4, `schema_version` stays 1 (an added variant loads old files), and the curve is not regenerated. The build proves that batch numbers under `unspecified` are unchanged on the fixtures. If any change, stop and report.
- **Docs.** ADR 0017 gains a Darkness bullet and a rejected option (done in this ticket). `CONTEXT.md`'s Lighting entry now lists Darkness. No new ADR.

Left out:

- Sight-gated targeting. Spells and abilities that target "a creature you can see" don't read sight, for darkness or for Invisible. It needs a field on every spell and in the SRD import, and one decision for every sight source. Moved to Not yet specified.

Build ticket: [Build darkness](../tickets/100-build-darkness.md).
