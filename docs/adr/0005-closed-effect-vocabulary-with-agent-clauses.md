# Abilities run from a closed effect vocabulary, and an ability can be partly modeled

The engine runs an ability only from a fixed list of effects (attack, save, damage, condition, modifier, roll modifier, forced move, heal, teleport, move-then-attack, multiattack, use-action and cast, plus usage limits, reaction triggers and passive triggers). Anything outside the list stays as text. The engine never interprets prose.

An ability is modeled, partly modeled, inert, or unmodeled. A partly modeled ability has an `effects` list plus `agent_clauses`, text the agent resolves while the engine runs the rest. An inert ability has no combat effect (Amphibious, Spider Climb) and keeps its text. An unmodeled ability has no effects and only its text. This refines the all-or-nothing split in the monster schema.

SRD imports ship a default resolution for each unmodeled ability or clause, in three classes. Neutral defaults count as having no effect in the fight. Base-form defaults keep a shape-shifter in its stat block form. Skipped defaults (spawn and splitting, persistent zones, objects with their own HP) are skipped, and the report warns that they can change the numbers. Homebrew abilities never get a default. Batch refuses until each one has a resolution.

## Considered options

- Every ability is fully modeled or fully unmodeled. Rejected because a lycanthropy bite or Mummy Rotting Fist would block the whole attack for one long-term curse clause.
- A scripting layer so a stat block can define new effect types. Rejected for phase 1. The map already defers agent-written engine code, and a closed list keeps batch and live results comparable.
- Parse the `desc` prose into effects at run time. Rejected because it would make the engine's results depend on wording, and the engine could not tell the user why it read a sentence a certain way.

## Consequences

- A new effect type is an engine change, a schema change and a version bump. Adding one is the way to model a new pattern.
- The SRD import fills the vocabulary from the structured fields where it can and from a hand-authored effects layer where the text carries the mechanic. The import review lists each mismatch.
- `recharge` usage accepts an optional `group` key (for example `breath_weapons` on the two breath actions of a metallic dragon). Abilities sharing a `group` on the same combatant share one recharge state and roll once at the start of the turn.
- Spells are modeled through an engine-owned spell effects file keyed by SRD spell id, not through the stat block. A spell id absent from the file makes the cast an unmodeled ability with its default resolution. The future spells map can replace the file.
- Zones, spawned creatures and objects with their own HP stay unmodeled. If early reports show many monsters hitting the skipped class, revisit this ADR before adding engine entities.
