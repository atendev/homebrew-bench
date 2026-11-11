# Map: homebrew-bench spell testing

## Destination

A spell-testing spec. Each homebrew spell is converted into an effects entry, checked, run in batch against same-level SRD spells, and run in live combat from both sides. The way to build it is a set of implementation tickets appended to this map by the final ticket.

## Notes

- Domain: D&D 5e homebrew spell testing. Vocabulary lives in `CONTEXT.md`. Design decisions live in `docs/adr/`, especially 0005 (closed effect vocabulary), 0014 (spell effects file) and 0004 (pending prompts for live mode).
- Skills every session should consult: `grilling` and `domain-modeling` for grilling tickets. Build tickets will call `tdd`, `codebase-design`, `domain-modeling` and `code-review`.
- Planning only. The final ticket creates the build tickets and appends them to this map. No ticket here writes engine code.
- Settled while charting (not yet tickets):
  - Intake is included. A pasted homebrew spell is converted into an effects entry, then a human reviews it before any run. Homebrew never gets a default: an entry that can't be modeled is refused.
  - The report compares a homebrew spell with SRD spells of the same level. The monster CR guidance does not apply to spells.
  - The effects file models a curated set of SRD spells per level, growing on demand (ADR 0014). Thin levels are recorded as thin data, not modeled in full up front.
  - Live mode lets both sides cast the spell: the monster side from a stat block, and the user's party through pending prompts.
- Spell-related fog from the archived map (`.plan/archive/homebrew-bench-map.md`) carries forward into Not yet specified.

## Decisions so far

<!-- one line per closed ticket: [title](closed/NNN-slug.md): gist -->

- [Spell value metric](closed/001-spell-value-metric.md): spell lift (win-rate and casualty change from swapping the spell onto the caster role) is the verdict; damage per cast is a diagnostic; rows per cast level and cantrip tier (ADR 0020)
- [Homebrew spell intake rules](closed/002-homebrew-spell-intake-rules.md): the agent writes each homebrew spell entry into its own `--spell` file, flags inferred values, refuses unmodelable or non-combat spells and SRD id clashes, and the user reviews and can edit it; entries gain `casting_time` (ADR 0021)

## Not yet specified

- Target caps on non-attack spells: Bless, Bane and similar spells take every target passed, so `extra_targets` on them is documentary. Decide whether a spell's target count is enforced by the engine or by the agent ([archived map](archive/homebrew-bench-map.md), Target caps on non-attack effects).
- Counter pricing: a limited counter costs nothing at the default `resource_cost` of 0, so the Mage reads easier at levels 2 to 4. Decide how a counter is priced against Shield and against the spell it stops ([archived map](archive/homebrew-bench-map.md), Counter pricing).
- Rounds per round: `rounds` durations lose one per creature's turn, so Shield and Haste last one turn. Decide whether durations count per round for spells, and what that does to batch cells ([archived map](archive/homebrew-bench-map.md), Rounds per round).
- Flat save bonuses on concentration and repeat saves: those saves add a die but no flat timed save modifier. Decide whether spell saves read flat bonuses, and how old saves are gated ([archived map](archive/homebrew-bench-map.md), Flat save bonuses on concentration and repeat saves).
- Casts before an encounter: a spell with `casting_time: long` reads "not measured" in batch ([Homebrew spell intake rules](closed/002-homebrew-spell-intake-rules.md)). Decide whether batch or live mode can cast a spell before initiative, and how that reads against the buff horizon.

## Out of scope

- Homebrew items, classes, subclasses, feats, species and backgrounds: separate future maps. The design should not block them ([archived map](archive/homebrew-bench-map.md), Out of scope).
- Monster testing: built and closed in the archived map. This map changes it only where a spell touches it.
- Non-combat play and the prose rules reference, as in the archived map.

## Frontier

- [Live spell casting prompts](tickets/003-live-spell-casting-prompts.md)
