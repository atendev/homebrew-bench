---
id: 105
title: Build See Invisible
type: task
mode: AFK
status: open
blocked_by: [100]
claimed: false
---

## Question

Build the See Invisible capability decided in [Seeing Invisible creatures by spell or trait](../closed/095-seeing-invisible-creatures.md).

## Notes

- Two carriers, one name: a unit trait effect `"see_invisible"` (like `no_provoke`) and a new `ModifierTarget` `{"see_invisible": null}` whose change is `none` (like `extra_action`). Schema stays 1.
- `fn sees` in `bench-core/src/engine.rs` reads either carrier: an Invisible combatant is seen by an observer with Blindsight, Truesight or See Invisible. Blinded and `darkness` lighting still block it.
- Add See Invisibility to `data/spells/effects.json`: a self-bearer `see_invisible` timed modifier, `{"hours": 1}`, not concentration. The Ethereal Plane clause is an agent clause. Don't add it to `data/party/standard-spells.json`.
- The batch policy reads sight through the shared rule, so it follows the engine. Both carriers score 0 as a plan. Policy stays v4. No curve input carries the capability.
- No new event text. The cast logs as any timed modifier does.
- Skill: list `see_invisible` in the batch conversion guidance for "sees invisible creatures" and "can see Invisible creatures" trait text, and say Truesight already covers it, so don't add both. In `references/live-mode.md`, replace "See Invisibility and a 'sees invisible creatures' trait are your judgment: remove the Invisible condition by override for that attack." with: "The engine reads See Invisible: a creature with a `see_invisible` trait or a See Invisibility modifier sees Invisible creatures for attacks, Frightened, Opportunity Attacks and ranged attacks in close combat (ADR 0017). It doesn't help a Blinded creature or one in Darkness. Cast See Invisibility through the engine when the caster's stat block has it. A creature that sees Invisible some other way, with no trait or spell for it on its block, is your judgment." The docs show only removing an `active_modifiers` entry by override, so the text doesn't offer an override that adds one.
- ADR 0017 and `CONTEXT.md` already describe the rule (ticket 095).
- Blocked by [100 Build darkness](100-build-darkness.md) because both edit `fn sees`.
- Skills: `tdd`, `codebase-design`, `domain-modeling`, `code-review`.

## Resolution

<Filled in at close.>
