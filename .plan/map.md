# Map: homebrew-bench spell testing

## Destination

A spell-testing spec. Each homebrew spell is converted into an effects entry, checked, run in batch against same-level SRD spells, and run in live combat from both sides. The way to build it is a set of implementation tickets appended to this map by the final ticket.

## Notes

- Domain: D&D 5e homebrew spell testing. Vocabulary lives in `CONTEXT.md`. Design decisions live in `docs/adr/`, especially 0005 (closed effect vocabulary), 0014 (spell effects file) and 0004 (pending prompts for live mode).
- Skills every session should consult: `grilling` and `domain-modeling` for grilling tickets. Build tickets will call `tdd`, `codebase-design`, `domain-modeling` and `code-review`.
- Planning tickets 001 to 004 wrote no engine code. Build tickets 114 to 127 do, each in one session, with the skills its Notes name.
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
- [Live spell casting prompts](closed/003-live-spell-casting-prompts.md): both sides cast through `bench action` on a stat block action, no new prompt; a read-only `bench spell` cast card shows the entry before the user picks target and slot; start-time checks tie cast section to `casting_time`; clauses logged after the counter window (ADR 0022)
- [Create build tickets and append them to the map](closed/004-create-build-tickets.md): 13 build tickets and 1 grilling ticket (114 to 127) in thin slices; Rounds per round becomes a grilling ticket that blocks the spell baseline; the other five fog items stay open

## Not yet specified

- Target caps on non-attack spells: Bless, Bane and similar spells take every target passed, so `extra_targets` on them is documentary. Decide whether a spell's target count is enforced by the engine or by the agent ([archived map](archive/homebrew-bench-map.md), Target caps on non-attack effects).
- Counter pricing: a limited counter costs nothing at the default `resource_cost` of 0, so the Mage reads easier at levels 2 to 4. Decide how a counter is priced against Shield and against the spell it stops ([archived map](archive/homebrew-bench-map.md), Counter pricing).
- Flat save bonuses on concentration and repeat saves: those saves add a die but no flat timed save modifier. Decide whether spell saves read flat bonuses, and how old saves are gated ([archived map](archive/homebrew-bench-map.md), Flat save bonuses on concentration and repeat saves).
- Casts before an encounter: a spell with `casting_time: long` reads "not measured" in batch ([Homebrew spell intake rules](closed/002-homebrew-spell-intake-rules.md)). Live mode refuses a `long` spell at start (ADR 0022). Decide whether batch can cast one before initiative, and how that reads against the buff horizon.
- SRD agent clauses on cast: live mode logs a homebrew entry's agent clauses on cast but not an SRD spell's, which the agent reads from `effects.json`. Decide whether to emit SRD clauses too, given that it changes every replay fingerprint ([Live spell casting prompts](closed/003-live-spell-casting-prompts.md)).

None of these blocks the destination. Each already has a defined reading, and none changes the committed spell baseline. Counter pricing and SRD agent clauses on cast are the next to ticket if the spell baseline review shows they matter.

## Out of scope

- Homebrew items, classes, subclasses, feats, species and backgrounds: separate future maps. The design should not block them ([archived map](archive/homebrew-bench-map.md), Out of scope).
- Monster testing: built and closed in the archived map. This map changes it only where a spell touches it.
- Non-combat play and the prose rules reference, as in the archived map.

## Frontier

- [Spell entry fields](tickets/114-spell-entry-fields.md)
- [Control party and spell panel](tickets/121-control-party-and-spell-panel.md)
- [Rounds per round for spell durations](tickets/124-rounds-per-round-for-spell-durations.md)

## Build tickets

Blocking in brackets.

- [Spell entry fields](tickets/114-spell-entry-fields.md)
- [Spell book](tickets/115-spell-book.md) [114]
- [Live start with homebrew spells](tickets/116-live-start-with-homebrew-spells.md) [115]
- [Live cast log for homebrew clauses](tickets/117-live-cast-log-for-homebrew-clauses.md) [116]
- [Cast card](tickets/118-cast-card.md) [115]
- [Skill: homebrew spell intake and live casting](tickets/119-skill-homebrew-spell-intake-and-live-casting.md) [116, 117, 118]
- [Batch homebrew clause resolution](tickets/120-batch-homebrew-clause-resolution.md) [115]
- [Control party and spell panel](tickets/121-control-party-and-spell-panel.md)
- [Spell lift runner](tickets/122-spell-lift-runner.md) [115, 120, 121]
- [Model more SRD cantrips](tickets/123-model-more-srd-cantrips.md) [114]
- [Rounds per round for spell durations](tickets/124-rounds-per-round-for-spell-durations.md) (grilling)
- [Spell baseline](tickets/125-spell-baseline.md) [122, 123, 124]
- [Spell lift report](tickets/126-spell-lift-report.md) [120, 125]
- [Skill: batch spell testing](tickets/127-skill-batch-spell-testing.md) [119, 126]
