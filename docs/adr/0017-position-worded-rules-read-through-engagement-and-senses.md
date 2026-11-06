# Position-worded rules read through engagement and senses

Several 2024 condition rules depend on where creatures stand or what they can see: a Prone target, a Paralyzed or Unconscious target's automatic critical hits, a Grappled attacker, a Frightened attacker, and Invisible on either side. ADR 0003 gave the engine no grid, so each needs a fixed reading. Decided in ticket 067.

- **"Within 5 feet" means engaged.** Two combatants are within 5 feet when either holds the other in its engagement list. The range band alone doesn't count, because a missing band defaults to engaged. Prone grants Advantage to an engaged attacker and Disadvantage to any other, and a hit on a Paralyzed or Unconscious target is a Critical Hit when the attacker is engaged with it.
- **Sight is one rule, with no ranges.** An observer sees a combatant unless that combatant is Invisible and the observer has neither Blindsight nor Truesight, or the observer is Blinded and has no Blindsight. Every sense reaches every band. Line of sight is otherwise assumed. Invisible attack effects, Frightened's line-of-sight clause and Opportunity Attack eligibility (amending ADR 0015) all use it.
- **"The grappler" and "the source of fear" are the condition's `source`.** A Grappled or Frightened condition without one applies against every target, as the rule would with no named source.
- **A combatant at 0 HP that is down has the Unconscious condition, and Unconscious includes Prone, for attack rolls.** The engine reads both at roll time and stores neither in `conditions`.
- **Ranged attacks in close combat read both rules.** Added in ticket 077. A ranged attack roll has Disadvantage when an enemy is within 5 feet of the attacker (engaged, recorded on either side), sees it by the sight rule, and can act. An enemy is a combatant on another side. One that is dead, down or has Incapacitated or a condition that includes it (Paralyzed, Petrified, Stunned, Unconscious) doesn't count. An attack with a range and no reach is always ranged here. A thrown weapon has both: it is a melee attack against a target the attacker is engaged with and a ranged attack against any other. A hit trigger's `ranged` filter still reads the range alone.
- **Darkness is one more reason not to see.** Decided in ticket 091. When the encounter's lighting is `darkness`, an observer with no Darkvision, Blindsight or Truesight sees no one. Every rule that reads sight follows it: attack rolls, Frightened, ranged attacks in close combat and Opportunity Attacks. Two combatants that can't see each other attack with Advantage and Disadvantage, which cancel. Nothing is stored in `conditions`. This darkness is nonmagical. Magical Darkness, which Darkvision can't see through, is nearly always a local area, so it stays the agent's call. Spell and ability targeting that says "a creature you can see" doesn't read sight yet, for darkness or Invisible.

## Considered options

- **Compare sense ranges with range bands** (Blindsight 10 ft reaches only the engaged band). Rejected: bands have no fixed width, and the map already puts sense ranges against bands out of scope. A creature with short Blindsight reads a little stronger against Invisible foes at range.
- **Use the engaged range band for "within 5 feet".** Rejected: the band defaults to engaged when unset, so unrelated combatants would read as adjacent.
- **Keep the old fixed list** (Prone always grants Advantage, Frightened and Invisible always apply). Rejected: ranged attacks against Prone targets and creatures with Blindsight or Truesight read wrong in every batch.
- **Give every combatant that can't see in darkness the Blinded condition**, stored or read at roll time. Rejected: Blinded gives Advantage to every attacker, even one that can't see the target either. The rule says Blinded "when trying to see something in that area", which is what the sight rule already answers. A stored condition would also have to be added and removed whenever the lighting changes.
- **Store the implied Unconscious and Prone conditions on the combatant.** Rejected: it changes state files, overrides and the digest to express something the attack code can read directly.

## Consequences

- A reach attacker 10 feet from a Prone target reads as within 5 feet. The agent overrides when it matters.
- Future position-worded rules (the "creature moving within N feet" reaction trigger, Darkness) should start from these readings.
- The batch tactics policy must read the same sources, or it scores attacks the engine rolls differently. Ticket 056 does that under policy v3.
