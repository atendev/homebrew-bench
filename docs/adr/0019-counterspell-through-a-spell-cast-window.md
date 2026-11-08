# Counterspell through a spell-cast window

Ticket 092 found one reaction trigger worth modeling beyond ADR 0018's two: a spell being cast. The Mage, Archmage and Lich can Counterspell a party Fireball or heal, so batch numbers move. Counterspell was all agent clauses, so batch never countered. Ticket 101 decided the design and the build is two follow-up tickets (110 and 111).

- **A third trigger.** A reaction entry may carry `trigger: {spell_cast: {caster: enemy}}`. The cast being held is a leveled spell cast by an enemy of the reactor. Cantrips and casts by allies open no window, a deliberate simplification: a cantrip is a legal Counterspell target, but the batch policy would never spend a counter on one. The entry still carries exactly one `trigger`, and the loader refuses `spell_cast` anywhere but a reaction.
- **The window holds the cast.** It follows ADR 0018: the held cast request, the dice so far and the answers so far are saved in a `reaction` pending prompt, and the request runs again from the state before the cast with the answer. The cast is held before its slot, usage and effects resolve. One window goes to each eligible reactor in initiative order. A reactor needs the shared checks (can act, no "no Reactions" modifier, reaction unspent) and a ready slot or usage for the reaction. The first taken counter that succeeds ends the cast and opens no later window. A failed save leaves the cast live, so later reactors are still asked, and each spends its own reaction and usage.
- **The counter effect.** A new effect `counter_spell`, additive at schema version 1. The caster makes a Constitution save against the reactor's spell save DC. On a failure the cast is cancelled and logged `spell_countered`. Counterspell is cast at its own level, 3, since the 2024 spell has no upcast.
- **What a countered cast costs.** The caster's action, bonus action or reaction stays spent. A spell slot is not expended, as the spell says. A per-day or innate usage does stay spent, since the rules are silent on non-slot usage.
- **Counters are not countered.** A cast made as a reaction (Counterspell, Shield) opens no spell-cast window. A window holds one command, and recursion would need nested held commands. Live mode can still run a counter-counter with `bench action`. This is a known limit.
- **Sight stays unread.** "A creature you can see" is not read by this trigger, as the map already leaves sight-gated targeting (ticket 091). A later sight ticket can cover all reactions at once. "Within 60 feet" reads as any combatant (ADR 0017).
- **The batch policy.** A reactor counters when its reaction is free and the expected value of the enemy cast (damage, healing and control it would deliver, by the existing scoring) beats the reaction reserve plus the slot or usage cost. The reserve term already covers holding the reaction for Shield. The rule is deterministic and logs a one-line reason through `reaction_reason`. It never counters a spell of value zero. The policy version moves up for it.
- **Protective Magic.** `per_day` usage gains an optional `group` key. Protective Magic splits into two reaction entries, Shield on the hit trigger and Counterspell on the spell-cast trigger, sharing one group so the Mage's and Archmage's 3/day stays one pool. The Lich's has no limit. The additive key leaves `schema_version` at 1.
- **The standard party.** The party caster gets no Counterspell, so the baseline curve stays comparable and only monsters under test change the numbers.
- **The gate.** Windows for this trigger open only when `settings.spell_cast_windows` is true. `bench start` and batch encounters set it, and encounters saved earlier leave it false. Reusing `settings.reaction_windows` would turn the casts in old saves into prompts that `undo` and `replay` drop. The engine version and the policy version both move, and the fixture-set and party fingerprints regenerate.
- **Log and public view.** A taken counter logs `REACTION <reactor>: Counterspell (cast by <caster>)`, then the caster's save, then `spell_countered` (slot not expended) or the cast resolving as normal. Window lines follow ADR 0018, and the public view hides a monster's window as it does today.

## Considered options

- **Batch-only special case with no window.** Rejected. Live mode would lose the window, and the policy could not resolve the save.
- **Counterspell stays agent clauses.** Rejected. Batch never counters, so the three monsters read easier than they play.
- **One entry carrying two triggers for Protective Magic.** Rejected. It breaks the one-trigger rule on a reaction entry.
- **Reusing `settings.reaction_windows`.** Rejected, as above.
- **Reading sight now.** Rejected. It would split the sight-gated targeting rule across one spell.
- **Counterspell for the party caster.** Rejected. It shifts the baseline curve for no tested monster.

## Consequences

- Build tickets 110 and 111 carry the engine and the policy and data work.
- Batch numbers for the Mage, Archmage and Lich change when 111 lands. Fingerprints regenerate then.
- ADR 0005 gains the third trigger and the `counter_spell` effect when 110 lands. CONTEXT.md is updated now.
