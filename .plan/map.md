# Map: homebrew-bench spell testing

## Destination

A spell-testing spec. Each homebrew spell is converted into an effects entry, checked, run in batch against same-level SRD spells, and run in live combat from both sides. The way to build it is a set of implementation tickets appended to this map by the final ticket.

## Notes

- Domain: D&D 5e homebrew spell testing. Vocabulary lives in `CONTEXT.md`. Design decisions live in `docs/adr/`, especially 0005 (closed effect vocabulary), 0014 (spell effects file) and 0004 (pending prompts for live mode).
- Skills every session should consult: `grilling` and `domain-modeling` for grilling tickets. Build tickets will call `tdd`, `codebase-design`, `domain-modeling` and `code-review`.
- Planning tickets 001 to 004 wrote no engine code. Build tickets 005 to 018 do, each in one session, with the skills its Notes name.
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
- [Create build tickets and append them to the map](closed/004-create-build-tickets.md): 13 build tickets and 1 grilling ticket (005 to 018) in thin slices; Rounds per round becomes a grilling ticket that blocks the spell baseline; the other five fog items stay open

- [Spell entry fields](closed/005-spell-entry-fields.md): entries require `casting_time`, declare `agent_clauses` and take `source: "homebrew"` with `pasted_source`; unknown keys fail to parse and `bench schema spell` exports the schema (engine 0.1.36)

- [Spell book](closed/006-spell-book.md): every lookup goes through a spell book of encounter homebrew then SRD; the `--spell` loader refuses id clashes and duplicates; `bench schema validate` takes `--spell` (engine 0.1.37)

- [Live start with homebrew spells](closed/007-live-start-with-homebrew-spells.md): `bench start --spell` copies entries into state with a fingerprint line, refuses misplaced, `long` and block-less attack or save casts, and warns on unknown ids; `--slot` is refused for monsters and needs `upcast: true` above a homebrew level (engine 0.1.38)

- [Live cast log for homebrew clauses](closed/008-live-cast-log-for-homebrew-clauses.md): a homebrew cast logs one `agent_clause` per clause after the window closes and before `upcast_clause`; a countered cast logs none; a monster's clause events are hidden in the public view (engine 0.1.39)

- [Cast card](closed/009-cast-card.md): `bench spell` prints the entry at its cast level through the engine's own scaling, with caster numbers from a live encounter or homebrew from `--spell`; live `bench action` refuses a wrong cast target count with the card's sentence (engine 0.1.40)

- [Skill: homebrew spell intake and live casting](closed/010-skill-homebrew-spell-intake-and-live-casting.md): a new `spell-intake.md` reference writes, flags, refuses and reviews entries, with flags stored in `notes`; live mode adds the cast action, `--spell` at start, the cast card before a cast, and the clause source per spell

## Not yet specified

- Target caps on non-attack spells: Bless, Bane and similar spells take every target passed, so `extra_targets` on them is documentary. Decide whether a spell's target count is enforced by the engine or by the agent ([archived map](archive/homebrew-bench-map.md), Target caps on non-attack effects).
- Counter pricing: a limited counter costs nothing at the default `resource_cost` of 0, so the Mage reads easier at levels 2 to 4. Decide how a counter is priced against Shield and against the spell it stops ([archived map](archive/homebrew-bench-map.md), Counter pricing).
- Flat save bonuses on concentration and repeat saves: those saves add a die but no flat timed save modifier. Decide whether spell saves read flat bonuses, and how old saves are gated ([archived map](archive/homebrew-bench-map.md), Flat save bonuses on concentration and repeat saves).
- Casts before an encounter: a spell with `casting_time: long` reads "not measured" in batch ([Homebrew spell intake rules](closed/002-homebrew-spell-intake-rules.md)). Live mode refuses a `long` spell at start (ADR 0022). Decide whether batch can cast one before initiative, and how that reads against the buff horizon.
- Declared homebrew upcasts: a stat block cast may declare `slot_level` above a homebrew entry's level with `upcast: false`, and it runs scaled because only `--slot` checks the flag ([Cast card](closed/009-cast-card.md)). Decide whether `bench start` refuses it.
- SRD agent clauses on cast: live mode logs a homebrew entry's agent clauses on cast but not an SRD spell's, which the agent reads from `effects.json`. Decide whether to emit SRD clauses too, given that it changes every replay fingerprint ([Live spell casting prompts](closed/003-live-spell-casting-prompts.md)).
- Intake flags on spell entries: entries have no `intake_flags` field, so intake writes flags into `notes` as text ([Skill: homebrew spell intake and live casting](closed/010-skill-homebrew-spell-intake-and-live-casting.md)). Decide whether entries gain the field so reports can list the flags.

None of these blocks the destination. Each already has a defined reading, and none changes the committed spell baseline. Counter pricing and SRD agent clauses on cast are the next to ticket if the spell baseline review shows they matter.

## Out of scope

- Homebrew items, classes, subclasses, feats, species and backgrounds: separate future maps. The design should not block them ([archived map](archive/homebrew-bench-map.md), Out of scope).
- Monster testing: built and closed in the archived map. This map changes it only where a spell touches it.
- Non-combat play and the prose rules reference, as in the archived map.

## Frontier

- [Batch homebrew clause resolution](tickets/011-batch-homebrew-clause-resolution.md)
- [Model more SRD cantrips](tickets/014-model-more-srd-cantrips.md)
- [Control party and spell panel](tickets/012-control-party-and-spell-panel.md)
- [Rounds per round for spell durations](tickets/015-rounds-per-round-for-spell-durations.md)

## Build tickets

Blocking in brackets.

- [Spell entry fields](closed/005-spell-entry-fields.md) (closed)
- [Spell book](closed/006-spell-book.md) (closed)
- [Live start with homebrew spells](closed/007-live-start-with-homebrew-spells.md) (closed)
- [Live cast log for homebrew clauses](closed/008-live-cast-log-for-homebrew-clauses.md) (closed)
- [Cast card](closed/009-cast-card.md) (closed)
- [Skill: homebrew spell intake and live casting](closed/010-skill-homebrew-spell-intake-and-live-casting.md) (closed)
- [Batch homebrew clause resolution](tickets/011-batch-homebrew-clause-resolution.md) [006]
- [Control party and spell panel](tickets/012-control-party-and-spell-panel.md)
- [Spell lift runner](tickets/013-spell-lift-runner.md) [006, 011, 012]
- [Model more SRD cantrips](tickets/014-model-more-srd-cantrips.md)
- [Rounds per round for spell durations](tickets/015-rounds-per-round-for-spell-durations.md) (grilling)
- [Spell baseline](tickets/016-spell-baseline.md) [013, 014, 015]
- [Spell lift report](tickets/017-spell-lift-report.md) [011, 016]
- [Skill: batch spell testing](tickets/018-skill-batch-spell-testing.md) [017]
