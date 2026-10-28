# SRD 5.2 monster ability survey

Findings for research ticket 008. Source: `data/srd/2024/5e-SRD-Monsters.json` only (SRD 5.2, upstream commit in `data/srd/COMMIT`). No 2014 data was read.

## Short answer

- The file holds 341 monster entries and 1,392 ability entries (354 traits, 838 actions, 89 bonus actions, 24 reactions, 87 legendary actions) under 422 distinct ability names. 18 of the 341 entries are form stat blocks (Vampire has three, and each of the five werecreatures has three), so the file holds 329 distinct creatures.
- Almost everything the engine cares about repeats. 157 of the 422 names appear on two or more monsters, and the rest (265) are one-offs, but most one-offs are still built from the same parts: an attack roll or a save, damage dice, a condition, a recharge, an area shape. Only 83 entries (6 percent) fall in the "leave unmodeled" bucket below, and only 2 entries match no pattern at all.
- The data is regular enough to model from fields, not text. Saves, attack rolls, damage and usage limits are structured (`dc`, `damage`, `attack_bonus`, `usage`, `spellcasting`, `actions`). Conditions, areas, forced movement, riders and "Failure:" and "Success:" branches live only in the `desc` text, and the text follows one house style, so a parser or a hand-authored effects layer can read it reliably.
- Suggested line for the effects vocabulary: model attack rolls, saves, damage, conditions, recharge and per-day limits, areas, multiattack, teleport and move-then-attack, healing, and a short list of flag traits. Leave unmodeled: shape-shifting, revival after death, creating creatures or objects, persistent zones, curses, possession, and the senses and telepathy traits. Details are in the last two sections.

## How I counted

Script: `scripts/survey-srd-abilities.py` (Python 3 standard library only). Run `python3 -I scripts/survey-srd-abilities.py` for the summary, or add `--md`, `--extras`, `--unmatched`, `--unmodeled`, `--prose` or `--sample ID`.

1. The script walks every monster and every entry in `special_abilities`, `actions`, `bonus_actions`, `reactions` and `legendary_actions`. One entry is one count. The five lists are the only places abilities live; the file has no `lair_actions` key.
2. Each entry is tagged with zero or more patterns. A tag is a rule over the structured fields (`usage.type`, `dc`, `damage`, `spellcasting`, `multiattack_type`, `actions`, `action_options`) and the `desc` text (regexes for "Saving Throw: DC", "Attack Roll", "Hit:", area shapes such as "15-foot Cone", condition names, "Failure:", "Success:", "pushed", "teleports" and so on), plus a short name list for a few traits that have no text signal (for example Spider Climb and Amphibious). Tags overlap on purpose: a recharging fire breath is a breath weapon, a recharge, a save for damage and a cone at once.
3. "Entries" is the number of ability entries with the tag. "Monsters" is the number of distinct monster entries that have at least one. Dragons inflate both: the same ability appears on wyrmling, young, adult and ancient entries, and each counts.
4. The "Kind" column says whether the pattern is mechanical or prose-only, using two checks. First, the share of matching entries whose structured fields or text carry something computable: a structured field, a dice expression, a save, an attack roll, advantage or disadvantage, a condition name, a damage or healing clause, a speed or teleport clause, an area shape, or a resistance clause. Second, a hand call on whether the pattern is a rule the engine would run (Mechanical), a switch the engine would only read (Flag), or something that depends on state outside a fight (stateful) or on judgment (Prose-only). The percentage in brackets is the first check.
5. I read samples of every rule's matches while building it and tightened rules that were noisy (for example, the condition rule no longer counts "unless the azer has the Incapacitated condition"). The counts are good to a few entries, not to the unit. Treat any count under about 10 as approximate.
6. The best class for an entry is the most modelable class among its tags, in the order effect, spell, flag, unmodeled. This is how the coverage numbers below are made.

Kind labels:

- Mechanical: rolls, saves, conditions, damage, recharge, movement or area shapes. The engine could run it from a structured effect.
- Mechanical, via spell data: the ability casts spells. The spell data in `data/srd/2024/5e-SRD-Spells.json` carries the mechanics.
- Flag: a rule modifier with no roll of its own, such as advantage on saves against spells. The engine reads a tag instead of running an effect.
- Mechanical but stateful: it has rules text, but running it needs state the fight does not otherwise track (new creatures, objects with hit points, zones that persist, long-term afflictions, death and revival).
- Prose-only: nothing the engine can compute; the agent reads it and rules on it.

## Data quirks that affect modeling

- Dual breath weapons lose their recharge. Silver, gold, copper, brass and bronze dragons have two breath actions that share one "Breath Weapons (Recharge 5-6)" header. Only the first breath carries `usage`; the second (Sleep, Paralyzing, Slowing, Weakening, Repulsion Breath) has no `usage` field. That is 20 entries (5 families, 4 sizes). A parser must carry the recharge across, or the data needs a correction layer.
- Recharge values: 88 recharge entries use `1d6`, with minimum 5 on 69 entries, 6 on 14 and 4 on 5. One entry recharges after a short or long rest (Cloaker Phantasms). Per-day limits: 61 entries; times 3 on 26, 4 on 15, 1 on 13, 2 on 6, 6 on 1; 34 of those are Legendary Resistance, so 27 are real per-day abilities.
- 309 monsters have no legendary actions. Only 32 monsters (dragons and a few others) have any, 87 entries in all. Legendary actions have no `usage`; the cooldown ("can't take this action again until the start of its next turn") is in the text of 44 entries.
- Attack rolls on 18 weak creatures (badger, bat, cat and so on) have no `damage` entry because the damage is a flat 1, which is not written as dice.
- Reactions are written as "Trigger: ... Response: ..." in `desc` (21 of 24). The trigger is not a structured field.
- `multiattack_type` is set on all 188 Multiattack entries as `actions` or `action_options`, and the component attacks are named in `actions[]` with a count. The `type` field of each component is `melee`, `ranged` or `special`; `special` points at a non-attack action.
- Spellcasting is a structured block (ability, DC, list of spells with usage) on 90 entries across 67 monsters. Another 23 legendary actions and a trait name a spell to cast ("uses Spellcasting to cast Fear") without a block.
- Static resistances and immunities are not abilities. They sit in stat block fields: 72 monsters have `damage_resistances`, 15 `damage_vulnerabilities`, 124 `damage_immunities`, 98 `condition_immunities`. Only a handful of traits grant or change resistance (see "Resistance and immunity toggles").
- Area shapes use a fixed phrasing: "15-foot Cone", "5-foot Emanation originating from the ghast", "60-foot-long, 10-foot-wide Line", "20-foot-radius Sphere centered on a point", "10-foot Cube", and "20-foot-radius, 60-foot-high Cylinder". A few entries drop a hyphen or space ("20foot Emanation", "5-footwide Line"); the script tolerates both.

## Pattern table

Counts are ability entries and the distinct monster entries that have them. Examples are written as Monster: Ability.


### Multiattack

| ID | Pattern | Entries | Monsters | Sections | Kind | Examples |
|---|---|---:|---:|---|---|---|
| MA1 | Multiattack: one attack repeated N times | 63 | 63 | action 63 | Mechanical (100%) | Air Elemental: Multiattack; Animated Armor: Multiattack; Black Dragon Wyrmling: Multiattack |
| MA2 | Multiattack: two or more different attacks in a fixed sequence | 30 | 30 | action 30 | Mechanical (100%) | Balor: Multiattack; Bearded Devil: Multiattack; Behir: Multiattack |
| MA3 | Multiattack with a choice between options (action_options) | 95 | 95 | action 95 | Mechanical (100%) | Aboleth: Multiattack; Assassin: Multiattack; Bandit Captain: Multiattack |
| MA4 | Multiattack that includes a non-attack action (special type) or a spell | 57 | 57 | action 57 | Mechanical (100%) | Aboleth: Multiattack; Behir: Multiattack; Adult Black Dragon: Multiattack |
| MA5 | Multiattack that lets the monster swap an attack for a spell or other action | 55 | 55 | action 55 | Mechanical (100%) | Adult Black Dragon: Multiattack; Ancient Black Dragon: Multiattack; Adult Blue Dragon: Multiattack |
| MA6 | Multiattack whose structured fields are empty (prose only) | 0 | 0 |  | none found |  |

### Attack roll

| ID | Pattern | Entries | Monsters | Sections | Kind | Examples |
|---|---|---:|---:|---|---|---|
| AT1 | Attack roll with one damage entry (or flat 1 damage) and no rider | 215 | 185 | action 215 | Mechanical (100%) | Air Elemental: Thunderous Slam; Animated Armor: Slam; Animated Flying Sword: Slash |
| AT2 | Attack roll with a second damage entry (extra damage of another type or dice) | 148 | 134 | action 148 | Mechanical (100%) | Animated Rug of Smothering: Smother; Ankheg: Bite; Assassin: Shortsword |
| AT3 | Attack roll that applies a condition on hit | 92 | 87 | action 92 | Mechanical (100%) | Aboleth: Tentacle; Animated Rug of Smothering: Smother; Ankheg: Bite |
| AT4 | Attack roll followed by a saving throw rider | 18 | 18 | action 18 | Mechanical (100%) | Bearded Devil: Infernal Glaive; Cockatrice: Petrifying Bite; Death Dog: Bite |
| AT5 | Attack roll with forced movement on hit | 1 | 1 | action 1 | Mechanical (100%) | Fire Giant: Hammer Throw |
| AT6 | Melee or Ranged attack roll (thrown weapon or both modes) | 23 | 23 | action 23 | Mechanical (100%) | Bugbear Stalker: Javelin; Bugbear Warrior: Light Hammer; Gladiator: Spear |
| AT7 | Attack roll with a curse or lasting affliction on hit (lycanthropy, mummy rot) | 14 | 14 | action 14 | Mechanical but stateful (100%) | Incubus: Restless Touch; Mummy: Rotting Fist; Rakshasa: Cursed Touch |
| AT8 | Attack roll that kills or converts a target reduced to 0 Hit Points | 4 | 4 | action 4 | Mechanical but stateful (100%) | Gibbering Mouther: Bite; Mummy: Rotting Fist; Vampire Familiar: Umbral Dagger |

### Saving throw

| ID | Pattern | Entries | Monsters | Sections | Kind | Examples |
|---|---|---:|---:|---|---|---|
| SV1 | Save for damage, half on success | 79 | 73 | action 66, trait 6, legendary action 4, bonus action 3 | Mechanical (100%) | Aboleth: Consume Memories; Air Elemental: Whirlwind; Ankheg: Acid Spray |
| SV2 | Save for damage, nothing on success | 53 | 51 | action 30, legendary action 14, bonus action 7, trait 1, reaction 1 | Mechanical (100%) | Aboleth: Mucus Cloud; Behir: Constrict; Behir: Swallow |
| SV3 | Save or condition (non-attack ability) | 83 | 72 | action 61, bonus action 11, legendary action 6, trait 4, reaction 1 | Mechanical (100%) | Aboleth: Dominate Mind; Air Elemental: Whirlwind; Basilisk: Petrifying Gaze |
| SV4 | Save with forced movement on failure | 10 | 10 | action 8, legendary action 2 | Mechanical (100%) | Air Elemental: Whirlwind; Bronze Dragon Wyrmling: Repulsion Breath; Bulette: Deadly Leap |
| SV5 | Save with a repeat save or end-of-turn save to end the effect | 26 | 26 | action 24, bonus action 2 | Mechanical (100%) | Aboleth: Dominate Mind; Basilisk: Petrifying Gaze; Brass Dragon Wyrmling: Sleep Breath |
| SV6 | Save where success grants immunity for a time | 20 | 19 | action 14, trait 3, bonus action 2, reaction 1 | Mechanical (100%) | Chain Devil: Unnerving Gaze; Cloaker: Moan; Ghast: Stench |
| SV7 | Save with a Failure or Success branch beyond damage and conditions (stat penalty, curse, transport, custom) | 15 | 15 | action 11, legendary action 2, trait 1, bonus action 1 | Mechanical but stateful (100%) | Copper Dragon Wyrmling: Slowing Breath; Gibbering Mouther: Gibbering; Gold Dragon Wyrmling: Weakening Breath |
| SV8 | Two-stage save ('First Failure' then 'Second Failure') | 9 | 9 | action 7, bonus action 2 | Mechanical (100%) | Basilisk: Petrifying Gaze; Cockatrice: Petrifying Bite; Death Dog: Bite |
| SV9 | Save with a damage or condition dependent on the target's current Hit Points or type (20 HP or fewer, Humanoid only, Undead excluded) | 13 | 13 | action 11, bonus action 1, legendary action 1 | Mechanical (100%) | Ghost: Horrific Visage; Ghoul: Claw; Gold Dragon Wyrmling: Weakening Breath |

### Area shape

| ID | Pattern | Entries | Monsters | Sections | Kind | Examples |
|---|---|---:|---:|---|---|---|
| AR1 | Cone | 55 | 47 | action 53, bonus action 2 | Mechanical (100%) | Basilisk: Petrifying Gaze; Brass Dragon Wyrmling: Sleep Breath; Bronze Dragon Wyrmling: Repulsion Breath |
| AR2 | Line | 24 | 24 | action 22, legendary action 2 | Mechanical (100%) | Ankheg: Acid Spray; Behir: Lightning Breath; Black Dragon Wyrmling: Acid Breath |
| AR3 | Emanation | 33 | 29 | trait 16, action 11, legendary action 4, bonus action 2 | Mechanical (100%) | Aboleth: Mucus Cloud; Azer Sentinel: Fire Aura; Balor: Death Throes |
| AR4 | Sphere | 9 | 9 | legendary action 6, action 3 | Mechanical (100%) | Adult Bronze Dragon: Thunderclap; Gibbering Mouther: Blinding Spittle; Adult Green Dragon: Noxious Miasma |
| AR5 | Cube | 3 | 3 | reaction 2, action 1 | Mechanical (100%) | Rust Monster: Destroy Metal; Giant Octopus: Ink Cloud; Mule: Ink Cloud |
| AR6 | Cylinder | 2 | 2 | action 2 | Mechanical (100%) | Djinni: Create Whirlwind; Storm Giant: Lightning Storm |
| AR7 | Any area template (union of the six shapes) | 126 | 88 | action 92, trait 16, legendary action 12, bonus action 4, reaction 2 | Mechanical (100%) | Aboleth: Mucus Cloud; Ankheg: Acid Spray; Azer Sentinel: Fire Aura |

### Usage limit

| ID | Pattern | Entries | Monsters | Sections | Kind | Examples |
|---|---|---:|---:|---|---|---|
| BR1 | Breath weapon (name ends in Breath, or desc says exhales) | 72 | 52 | action 72 | Mechanical (100%) | Behir: Lightning Breath; Black Dragon Wyrmling: Acid Breath; Brass Dragon Wyrmling: Fire Breath |
| US1 | Recharge on a die roll (Recharge 5-6, 6, 4-6) | 88 | 88 | action 72, bonus action 15, reaction 1 | Mechanical (100%) | Air Elemental: Whirlwind; Ankheg: Acid Spray; Basilisk: Petrifying Gaze |
| US2 | Limited uses per day | 61 | 56 | trait 35, bonus action 11, action 10, reaction 5 | Mechanical (100%) | Aboleth: Legendary Resistance; Aboleth: Dominate Mind; Couatl: Divine Aid |
| US3 | Recharge after a short or long rest | 1 | 1 | bonus action 1 | Mechanical (100%) | Cloaker: Phantasms |
| US4 | Legendary action cooldown ('can't take this action again until the start of its next turn') | 44 | 29 | legendary action 44 | Mechanical (95%) | Adult Black Dragon: Cloud of Insects; Adult Black Dragon: Frightful Presence; Adult Blue Dragon: Cloaked Flight |
| US5 | Prose-only usage limit with no usage field (once per turn, until a rest) | 4 | 4 | trait 3, action 1 | Mechanical but stateful (100%) | Djinni: Create Whirlwind; Green Hag: Coven Magic; Night Hag: Coven Magic |

### Spellcasting

| ID | Pattern | Entries | Monsters | Sections | Kind | Examples |
|---|---|---:|---:|---|---|---|
| SP1 | Spellcasting block with a structured spell list (spells, slots, or at-will lists) | 90 | 67 | action 60, bonus action 17, legendary action 7, trait 3, reaction 3 | Mechanical, via spell data (100%) | Adult Black Dragon: Spellcasting; Cloaker: Phantasms; Cloud Giant: Misty Step |
| SP2 | Names a spell to cast through a trait, action or legendary action without its own list | 23 | 19 | legendary action 22, trait 1 | Mechanical, via spell data (100%) | Adult Black Dragon: Frightful Presence; Adult Blue Dragon: Cloaked Flight; Adult Blue Dragon: Sonic Boom |

### Condition inflicted

| ID | Pattern | Entries | Monsters | Sections | Kind | Examples |
|---|---|---:|---:|---|---|---|
| CN-Grap | Inflicts Grappled | 38 | 37 | action 36, bonus action 1, trait 1 | Mechanical (100%) | Aboleth: Tentacle; Ankheg: Bite; Barbed Devil: Claws |
| CN-Rest | Inflicts Restrained | 34 | 28 | action 28, bonus action 6 | Mechanical (100%) | Animated Rug of Smothering: Smother; Basilisk: Petrifying Gaze; Behir: Constrict |
| CN-Pron | Inflicts Prone | 57 | 54 | action 51, bonus action 5, legendary action 1 | Mechanical (100%) | Air Elemental: Whirlwind; Balor: Flame Whip; Behir: Swallow |
| CN-Inca | Inflicts Incapacitated | 15 | 15 | action 13, legendary action 2 | Mechanical (100%) | Brass Dragon Wyrmling: Sleep Breath; Cloud Giant: Thundercloud; Ghost: Possession |
| CN-Pois | Inflicts Poisoned | 26 | 26 | action 23, trait 2, legendary action 1 | Mechanical (100%) | Assassin: Shortsword; Bearded Devil: Beard; Bone Devil: Infernal Sting |
| CN-Frig | Inflicts Frightened | 14 | 14 | action 9, bonus action 2, trait 2, reaction 1 | Mechanical (100%) | Chain Devil: Unnerving Gaze; Cloaker: Moan; Doppelganger: Unsettling Visage |
| CN-Blin | Inflicts Blinded | 16 | 16 | action 9, bonus action 4, legendary action 2, reaction 1 | Mechanical (100%) | Animated Rug of Smothering: Smother; Behir: Swallow; Cloaker: Attach |
| CN-Para | Inflicts Paralyzed | 18 | 15 | action 12, trait 6 | Mechanical (100%) | Chuul: Paralyzing Tentacles; Ghast: Claw; Lich: Paralyzing Touch |
| CN-Unco | Inflicts Unconscious | 7 | 7 | action 6, bonus action 1 | Mechanical (100%) | Brass Dragon Wyrmling: Sleep Breath; Homunculus: Bite; Incubus: Nightmare |
| CN-Char | Inflicts Charmed | 5 | 5 | action 5 | Mechanical (100%) | Aboleth: Dominate Mind; Harpy: Luring Song; Lamia: Corrupting Touch |
| CN-Deaf | Inflicts Deafened | 5 | 5 | action 3, legendary action 2 | Mechanical (100%) | Adult Bronze Dragon: Thunderclap; Storm Giant: Thunderbolt; Tarrasque: Thunderous Bellow |
| CN-Petr | Inflicts Petrified | 4 | 4 | bonus action 2, action 2 | Mechanical (100%) | Basilisk: Petrifying Gaze; Cockatrice: Petrifying Bite; Gorgon: Petrifying Breath |
| CN-Stun | Inflicts Stunned | 2 | 2 | action 2 | Mechanical (100%) | Otyugh: Tentacle Slam; Vrock: Stunning Screech |
| CN-Invi | Inflicts Invisible | 2 | 2 | trait 1, bonus action 1 | Mechanical (100%) | Invisible Stalker: Invisibility; Will-o’-Wisp: Vanish |
| CN-Exha | Inflicts Exhaustion levels | 3 | 3 | legendary action 2, trait 1 | Mechanical (100%) | Sphinx of Lore: Weight of Years; Troll: Loathsome Limbs; Sphinx of Valor: Weight of Years |

### Grapple and swallow

| ID | Pattern | Entries | Monsters | Sections | Kind | Examples |
|---|---|---:|---:|---|---|---|
| GR1 | Grapple with an escape DC (on hit or on failed save) | 39 | 38 | action 37, bonus action 1, trait 1 | Mechanical (100%) | Aboleth: Tentacle; Animated Rug of Smothering: Smother; Ankheg: Bite |
| GR2 | Swallow or engulf | 9 | 9 | action 5, bonus action 4 | Mechanical (100%) | Behir: Swallow; Gelatinous Cube: Engulf; Kraken: Swallow |
| GR3 | Ability that works on an already Grappled or Restrained target | 20 | 18 | action 12, bonus action 5, trait 3 | Mechanical (90%) | Aboleth: Consume Memories; Ankheg: Bite; Barbed Devil: Barbed Hide |

### Summon, spawn and animate

| ID | Pattern | Entries | Monsters | Sections | Kind | Examples |
|---|---|---:|---:|---|---|---|
| SM1 | Creates or animates other creatures (Split, Create Specter, Animate Trees, spawn) | 17 | 17 | action 14, reaction 2, trait 1 | Mechanical but stateful (94%) | Black Pudding: Split; Treant: Animate Trees; Troll Limb: Troll Spawn |
| OB1 | Creates an object or hazard with its own AC and HP (webs, tentacles, ropes, bags) | 6 | 6 | trait 3, action 3 | Mechanical but stateful (100%) | Erinyes: Magic Rope; Ettercap: Web Strand; Mummy Lord: Undead Restoration |

### Forced movement

| ID | Pattern | Entries | Monsters | Sections | Kind | Examples |
|---|---|---:|---:|---|---|---|
| FM1 | Pushes, pulls or drags a creature (any source) | 15 | 14 | action 11, legendary action 2, bonus action 1, reaction 1 | Mechanical (93%) | Air Elemental: Whirlwind; Bronze Dragon Wyrmling: Repulsion Breath; Bulette: Deadly Leap |

### Aura and zone

| ID | Pattern | Entries | Monsters | Sections | Kind | Examples |
|---|---|---:|---:|---|---|---|
| AU1 | Passive emanation aura on a trait (acts on creatures near the monster each turn) | 17 | 16 | trait 17 | Mechanical (100%) | Aboleth: Mucus Cloud; Azer Sentinel: Fire Aura; Balor: Death Throes |
| AU2 | Zone placed in the world by an action (whirlwind, cloud, ink) | 10 | 10 | action 8, reaction 2 | Mechanical but stateful (100%) | Copper Dragon Wyrmling: Slowing Breath; Darkmantle: Darkness Aura; Djinni: Create Whirlwind |
| AU3 | Retaliation or equipment damage when a creature touches or hits the monster | 4 | 4 | trait 4 | Mechanical (100%) | Barbed Devil: Barbed Hide; Black Pudding: Corrosive Form; Fire Elemental: Fire Form |

### Defense

| ID | Pattern | Entries | Monsters | Sections | Kind | Examples |
|---|---|---:|---:|---|---|---|
| DF1 | Magic Resistance (advantage on saves against spells) | 35 | 35 | trait 35 | Flag (97% computable) | Balor: Magic Resistance; Rakshasa: Greater Magic Resistance; Barbed Devil: Magic Resistance |
| DF2 | Legendary Resistance (turn a failed save into a success, per day) | 34 | 34 | trait 34 | Mechanical (100%) | Aboleth: Legendary Resistance; Balor: Legendary Resistance; Adult Black Dragon: Legendary Resistance |
| DF4 | Healing: regeneration, absorb damage to heal, drain to heal | 19 | 17 | trait 14, action 3, legendary action 1, bonus action 1 | Mechanical (100%) | Aboleth: Psychic Drain; Clay Golem: Acid Absorption; Flesh Golem: Lightning Absorption |
| DF5 | Revival: returns after being destroyed (Restoration, Spirit Jar) | 25 | 25 | trait 25 | Mechanical but stateful (100%) | Aboleth: Eldritch Restoration; Balor: Death Throes; Barbed Devil: Diabolical Restoration |
| DF6 | Death trigger: burst, instant effect, or stays at 1 HP (Death Burst, Death Throes, Undead Fortitude) | 8 | 8 | trait 8 | Mechanical (100%) | Balor: Death Throes; Magmin: Death Burst; Zombie: Undead Fortitude |
| DF7 | Reaction that boosts AC, reduces damage, redirects or negates an attack | 11 | 11 | reaction 11 | Mechanical (100%) | Bandit Captain: Parry; Mummy Lord: Whirlwind of Sand; Pirate Captain: Riposte |
| DF8 | Form-based immunity or defense by trait name (Immutable Form, Reflective Carapace, Reactive Heads, Blurred Form) | 10 | 9 | trait 10 | Flag (50% computable) | Assassin: Evasion; Clay Golem: Immutable Form; Hydra: Multiple Heads |
| DF9 | Temporary Hit Points or AC bonus granted to a creature | 9 | 9 | trait 7, bonus action 1, legendary action 1 | Mechanical (100%) | Swarm of Crawling Claws: Swarm; Frost Giant: War Cry; Unicorn: Shimmering Shield |

### Action chaining

| ID | Pattern | Entries | Monsters | Sections | Kind | Examples |
|---|---|---:|---:|---|---|---|
| AC1 | Legendary action, reaction or bonus action that just runs another of the monster's own actions | 13 | 11 | legendary action 13 | Mechanical (54%) | Aboleth: Lash; Adult Black Dragon: Frightful Presence; Adult Blue Dragon: Sonic Boom |

### Roll modifier

| ID | Pattern | Entries | Monsters | Sections | Kind | Examples |
|---|---|---:|---:|---|---|---|
| AD1 | Conditional advantage or disadvantage in a trait (Pack Tactics, Sunlight Sensitivity, Blood Frenzy) | 43 | 42 | trait 43 | Mechanical (100%) | Berserker: Bloodied Frenzy; Cloaker: Light Sensitivity; Commoner: Training |
| AD2 | Damage multiplier or conditional damage bonus (Siege Monster, Bloodied Fury, Berserk) | 10 | 10 | trait 10 | Mechanical (100%) | Berserker: Bloodied Frenzy; Clay Golem: Berserk; Earth Elemental: Siege Monster |

### Movement

| ID | Pattern | Entries | Monsters | Sections | Kind | Examples |
|---|---|---:|---:|---|---|---|
| MV1 | Self teleport (Teleport, Misty Step, Tree Stride, Radiant Teleport, Ethereal Jaunt) | 22 | 20 | bonus action 10, legendary action 4, action 3, trait 3, reaction 2 | Mechanical (95%) | Balor: Teleport; Cloud Giant: Misty Step; Dryad: Tree Stride |
| MV2 | Move then attack or move with a rider (Pounce, Charge, Trample, Rampage, Swoop) | 40 | 39 | legendary action 26, bonus action 13, action 1 | Mechanical (100%) | Adult Black Dragon: Pounce; Adult Blue Dragon: Cloaked Flight; Centaur Trooper: Trampling Charge |
| MV3 | Bonus-action Disengage, Hide, Dash or leap (Nimble Escape, Cunning Action, Leap) | 20 | 20 | bonus action 20 | Mechanical (20%) | Assassin: Cunning Action; Bulette: Leap; Clay Golem: Hasten |
| MV4 | Passive terrain or space traversal (Spider Climb, Web Walker, Ice Walk, Earth Glide, Tunneler, Standing Leap) | 57 | 49 | trait 57 | Flag (30% computable) | Air Elemental: Air Form; Ankheg: Tunneler; Black Pudding: Amorphous |
| MV5 | No Opportunity Attack when moving (Flyby, Agile) | 9 | 9 | trait 9 | Flag (100% computable) | Gargoyle: Flyby; Deer: Agile; Hippogriff: Flyby |
| MV6 | Speed change imposed on a target (Slowed, Speed halved or 0) | 8 | 8 | legendary action 4, action 4 | Mechanical (100%) | Adult Brass Dragon: Scorching Sands; Copper Dragon Wyrmling: Slowing Breath; Adult White Dragon: Freezing Burst |

### Transformation

| ID | Pattern | Entries | Monsters | Sections | Kind | Examples |
|---|---|---:|---:|---|---|---|
| TF1 | Shape-Shift and alternate forms (Shape-Shift, Succubus Form, Misty Escape) | 31 | 27 | bonus action 22, trait 6, action 3 | Prose-only (32%) | Doppelganger: Shape-Shift; Half-Dragon: Draconic Origin; Incubus: Succubus Form |
| TF2 | Invisibility, ethereal travel, or hiding (Invisibility, Vanish, Shadow Stealth) | 15 | 15 | bonus action 8, action 4, legendary action 2, trait 1 | Mechanical (93%) | Adult Blue Dragon: Cloaked Flight; Ghost: Etherealness; Imp: Invisibility |
| TF3 | Possession or mind-control of a target | 4 | 4 | action 4 | Mechanical but stateful (75%) | Aboleth: Dominate Mind; Ghost: Possession; Wight: Life Drain |

### Utility

| ID | Pattern | Entries | Monsters | Sections | Kind | Examples |
|---|---|---:|---:|---|---|---|
| UT1 | Breathing and environment (Amphibious, Water Breathing, Hold Breath) | 47 | 47 | trait 47 | Flag (0% computable) | Aboleth: Amphibious; Hydra: Hold Breath; Sahuagin Warrior: Limited Amphibiousness |
| UT2 | Light emission or light sensitivity (Illumination, Sunlight Sensitivity) | 18 | 18 | trait 16, bonus action 2 | Flag (67% computable) | Azer Sentinel: Illumination; Cloaker: Light Sensitivity; Drider: Sunlight Sensitivity |
| UT3 | Telepathy, extra senses, speech and communication traits | 18 | 18 | trait 18 | Prose-only (28%) | Aboleth: Probing Telepathy; Chuul: Sense Magic; Couatl: Shielded Mind |
| UT4 | Entry, placement and vulnerability restrictions (Forbiddance, Running Water, Stake to the Heart, Water Susceptibility) | 18 | 6 | trait 18 | Flag (56% computable) | Fire Elemental: Water Susceptibility; Flesh Golem: Aversion to Fire; Vampire Spawn: Vampire Weakness |
| UT5 | Bespoke one-off rules with no recurring mechanic (Magic Rope, Wishes, Spirit Jar, Soul Bag, Spell Storing) | 23 | 21 | trait 23 | Mechanical but stateful (87%) | Bugbear Stalker: Abduct; Djinni: Wishes; Earth Elemental: Siege Monster |

### Structure

| ID | Pattern | Entries | Monsters | Sections | Kind | Examples |
|---|---|---:|---:|---|---|---|
| RX1 | Reaction or ability written as 'Trigger: ... Response: ...' | 21 | 21 | reaction 21 | Mechanical (81%) | Bandit Captain: Parry; Black Pudding: Split; Chain Devil: Unnerving Gaze |

## Coverage by section

Best class per entry, after tagging (E effect, S spell, F flag, U unmodeled):

| Section | Entries | Effect | Spell | Flag | Unmodeled |
|---|---:|---:|---:|---:|---:|
| Trait | 354 | 134 | 4 | 165 | 51 |
| Action | 838 | 781 | 51 | 0 | 6 |
| Bonus action | 89 | 66 | 0 | 1 | 22 |
| Reaction | 24 | 19 | 1 | 0 | 4 |
| Legendary action | 87 | 80 | 7 | 0 | 0 |
| All | 1,392 | 1,080 | 63 | 166 | 83 |

Reading it: actions are nearly all effect-shaped (attacks, saves, multiattack, breath). Traits are the other half of the problem: 165 of 354 are flags with no roll of their own (Amphibious, Spider Climb, Magic Resistance, light sensitivity), and 51 are unmodeled (revival, senses, bespoke rules). Bonus actions are mostly effect-shaped, and all 22 of the unmodeled ones are Shape-Shift.

## Patterns by theme

### Save-or-condition and save-for-damage

205 entries (15 percent of all) use a saving throw, written "Constitution Saving Throw: DC 14, <targets>. Failure: ... Success: ...". Without an attack roll, 83 inflict a condition (SV3) and 132 deal damage (SV1 and SV2: 79 half on success, 53 nothing on success); many do both, so these overlap. Another 18 are an attack roll followed by a save rider (AT4, such as Cockatrice Petrifying Bite, Ghast Claw, Death Dog Bite). Another 92 attack-roll abilities inflict a condition on hit with no save (AT3, such as Aboleth Tentacle, Ankheg Bite).

Conditions inflicted, by entries (any source): Prone 57, Grappled 38, Restrained 34, Poisoned 26, Paralyzed 18, Blinded 16, Incapacitated 15, Frightened 14, Unconscious 7, Charmed 5, Deafened 5, Petrified 4, Exhaustion levels 3, Stunned 2, Invisible 2. Stunned is rare (Otyugh Tentacle Slam, Vrock Stunning Screech); Charmed is a handful of control abilities (Aboleth Dominate Mind, Harpy Luring Song, Lamia Corrupting Touch).

Save riders that recur and need a small amount of structure in an effect:

- Repeat or end-of-turn save to end the effect (SV5, 26 entries): Aboleth Dominate Mind, Basilisk Petrifying Gaze, the wyrmling Sleep Breath.
- Two-stage save, "First Failure" then "Second Failure" (SV8, 9 entries): Basilisk Petrifying Gaze, Cockatrice Petrifying Bite, Death Dog Bite, Silver Dragon Paralyzing Breath.
- Success grants immunity for 24 hours (SV6, 20 entries): Ghast Stench, Cloaker Moan, Ghost Horrific Visage, Pit Fiend Fear Aura.
- Outcome gated on the target's Hit Points or type (SV9, 13 entries): Ghoul Claw, Ghost Horrific Visage, Sea Hag Death Glare ("20 Hit Points or fewer"), Sprite Heart Sight (Celestials, Fiends and Undead automatically fail).
- Failure branch that is neither damage nor a standard condition (SV7, 15 entries; unmodeled): Copper Dragon Slowing Breath (no Reactions, Speed halved), Gold Dragon Weakening Breath (Disadvantage on Strength-based rolls), Sphinx Weight of Years (Exhaustion plus a rider).

All of this is mechanical. A few failure branches end in a free-text clause the engine would leave to the agent.

### Recharge and breath weapons

88 entries recharge on a die roll; every one is `1d6` (minimum 5 on 69, 6 on 14, 4 on 5). 72 entries are breath weapons across 52 monsters (dragons, mephits, hell hounds, chimera, behir, gorgon, dragon turtle, iron golem, winter wolf). 52 of the 72 carry `recharge on roll`; the other 20 are the second breath on a dual-breath dragon, which has lost its `usage` field (see quirks). 36 recharging abilities are not breath weapons: Air Elemental Whirlwind, Ankheg Acid Spray, Basilisk Petrifying Gaze, Blink Dog and Marilith Teleport, Medusa Petrifying Gaze, Clay Golem Hasten, Vampire Charm, and others. 64 of the 88 recharge entries have an area and 74 involve a saving throw.

Area shapes in the abilities, by entries: Cone 55, Emanation 33, Line 24, Sphere 9, Cube 3, Cylinder 2; 126 entries (88 monsters) use any area. Cones come in 15, 30, 60, 90 and 150 feet; emanations run from 5 to 500 feet (Harpy Luring Song at 300, Sphinx of Lore Mind-Rending Roar); lines and spheres appear mainly on dragons and legendary actions. Every area is mechanical and shaped by the same phrasing, so one `area` field (shape, size, origin) covers all six.

Per-day limits (61 entries, 27 not counting Legendary Resistance) and one rest recharge cover the rest of the usage cases. 4 entries limit use in prose with no `usage` field (Djinni Create Whirlwind, Coven Magic on three hags).

### Multiattack variants

188 Multiattack entries, one per monster that has one. By shape:

- One attack repeated N times, no choices (MA1, 63): Air Elemental, Animated Armor, wyrmlings.
- Several different attacks in a fixed sequence (MA2, 30): Balor, Bearded Devil, Behir.
- A choice between options, via `action_options` (MA3, 95): Aboleth (two Tentacle attacks, then Consume Memories or Dominate Mind), Assassin, Bandit Captain.
- Includes a non-attack action or a spell (MA4, 57): Chain Devil (two Chain attacks and Conjure Infernal Chain), Erinyes (three Withering Sword attacks and can use Entangling Rope), Aboleth.
- Lets the monster swap an attack for another action (MA5, 55): dragons ("It can replace one attack with a use of Spellcasting").

Shapes overlap: MA4 and MA5 mostly sit inside MA3, and a monster can be in more than one. The structured `actions` list is present on all 188, so a multiattack effect can be built from data, with the option and swap text as the only prose clauses to hand-author. Nothing here is prose-only.

### Damage riders and attack rolls

440 abilities (32 percent) are attack rolls, matching the 440 entries with `attack_bonus`. Shapes:

- One damage entry, nothing else (AT1, 215): Air Elemental Thunderous Slam, Animated Armor Slam, Axe Beak Beak. This includes 18 weak creatures whose damage is a flat 1 with no dice and no `damage` entry.
- A second damage entry of a different type or dice (AT2, 148): Ankheg Bite, Assassin Shortsword, Bone Devil Infernal Sting (piercing plus poison, then Poisoned).
- A condition on hit (AT3, 92), usually Grappled, Prone, Poisoned or Restrained.
- A save rider (AT4, 18) and a curse or lasting affliction (AT7, 14: lycanthropy bites, Mummy Rotting Fist, Rakshasa Cursed Touch, Incubus Restless Touch).
- Melee or ranged in one entry (AT6, 23): Bugbear Javelin, Gladiator Spear, Goblin Minion Dagger.
- Kills or converts on reaching 0 Hit Points (AT8, 4): Gibbering Mouther Bite, Mummy Rotting Fist, Vampire Familiar Umbral Dagger.

AT1 through AT6 are mechanical. AT7 and AT8 are mechanical text but need state the fight does not track (a long-term curse, a creature changing sides).

### Grappling, restraining, swallowing

- Grapple with an escape DC (GR1, 39 entries, 38 monsters): Aboleth Tentacle, Ankheg Bite, Barbed Devil Claws, Behir Constrict.
- Swallow or engulf (GR2, 9): Behir, Kraken, Purple Worm, Remorhaz, Tarrasque (bonus-action Swallow), Giant Frog, Giant Toad, Gelatinous Cube Engulf, Shambling Mound Engulf. Swallowed targets get Blinded and Restrained, Total Cover, and take repeating acid damage; the swallower holds one at a time.
- Abilities that need a grappled or restrained target (GR3, 20): Aboleth Consume Memories, Ankheg Bite, Barbed Devil Barbed Hide, Behir Swallow, Bugbear Quick Grapple and Abduct, Ettercap Reel.

All mechanical, and all use the Grappled and Restrained conditions plus an escape DC.

### Auras, zones, death effects and retaliation

- Trait with an Emanation (AU1, 17 entries): 11 are true auras applied each turn (Aboleth Mucus Cloud, Azer, Balor, Fire Elemental and Salamander Fire Aura, Ghast and Hezrou Stench, Hobgoblin Captain Aura of Authority, Pit Fiend Fear Aura, Remorhaz Heat Aura, Gibbering Mouther Aberrant Ground) and 6 are on-death bursts (Death Burst on four mephits and Magmin, Balor Death Throes).
- Death triggers (DF6, 8): the six bursts above plus Zombie and Ogre Zombie Undead Fortitude.
- Zones an action leaves in the world (AU2, 10 entries, stateful): Copper Dragon Slowing Breath, Darkmantle Darkness Aura, Djinni Create Whirlwind, Harpy Luring Song (concentration), Giant Octopus and Mule Ink Cloud. These need an area that persists, can be entered, and ends on a condition. Leave unmodeled until the engine tracks persistent zones.
- Retaliation or equipment damage when touched or hit (AU3, 4): Barbed Devil Barbed Hide, Black Pudding Corrosive Form, Fire Elemental Fire Form.

### Summons, spawn and objects

Nothing in the monster abilities calls other creatures from outside the fight. The creature-making abilities are spawn and splitting (SM1, 17 entries): Black Pudding and Ochre Jelly Split (reaction), Treant Animate Trees, Wraith Create Specter, Troll Limb Troll Spawn, Vampire and werecreature bites that create spawn or curses. Real summons exist only through spells in `spellcasting` blocks: Animate Dead (Lich, Mummy Lord), Create Undead (Ancient Black Dragon), Find Familiar (three hags). Objects with their own AC and HP appear six times (OB1): Giant Spider Web, Ettercap Web Strand, Roper Tentacle, Erinyes Magic Rope, Night Hag Soul Bag, Mummy Lord heart. All are mechanical in text but need engine entities; treat as unmodeled.

### Forced movement

15 entries (14 monsters): Bronze Dragon Repulsion Breath, Silver Dragon Cold Gale, Air Elemental Whirlwind, Bulette Deadly Leap, Chain Devil Conjure Infernal Chain, Fire Giant Hammer Throw (on an attack roll), Ettercap and Roper Reel (pull), Shambling Mound Engulf, Goblin Boss Redirect Attack (swap places). 10 sit behind a save (SV4). The phrasing is "pushed up to N feet away from the dragon" or "pulled N feet straight toward"; one `forced_move` effect (direction, distance) fits all.

### Resistance and immunity toggles

Static damage and condition resistances, immunities and vulnerabilities are stat block fields, not abilities (72, 124, 15 and 98 monsters). Inside abilities, the recurring forms are few:

- Magic Resistance (DF1, 35 entries, 35 monsters): advantage on saves against spells and magical effects. Flag.
- Legendary Resistance (DF2, 34 entries): per-day (3, or 4 in lair) override of a failed save. Mechanical; needs a counter.
- Absorb and regenerate (DF4, 19): Acid, Fire and Lightning Absorption on golems and Shambling Mound (immune and heal), Regeneration on Troll, Oni, Shield Guardian, drain-to-heal (Aboleth Psychic Drain, Will-o'-Wisp Consume Life, vampire bites).
- Conditional resistance: Nightmare Confer Fire Resistance, Vampire mist form (Resistance to all damage but sunlight), Dragon Turtle (underwater does not grant Resistance to its fire). All one-offs.
- Form immunity traits (DF8, 10): Immutable Form on three golems, Evasion, Reactive Heads, Blurred Form (Disadvantage on attacks against it), Multiple Heads. Flags.
- Temporary Hit Points and AC bonuses granted to others (DF9, 9): Frost Giant War Cry, Unicorn Shimmering Shield.
- Reactions that mitigate an attack (DF7, 11): Parry on seven monsters (Bandit Captain, Erinyes, Gladiator, Knight, Marilith, Noble, Warrior Veteran), Riposte, Deflect Missile, Protection, Whirlwind of Sand.

### Movement and positioning

- Teleport and equivalents (MV1, 22 entries): Balor, Marilith and Nalfeshnee Teleport, Cloud Giant and others Misty Step, Dryad Tree Stride, Phase Spider Ethereal Jaunt.
- Move and attack as one action (MV2, 40): dragon Pounce (18 legendary actions), Centaur Trampling Charge, Gnoll Rampage, Tarrasque World-Shaking Movement.
- Bonus-action Disengage, Hide, Dash or leap (MV3, 20): Goblin Nimble Escape (6), Assassin and Spy Cunning Action, Bulette Leap, Clay Golem Hasten.
- Passive traversal and size (MV4, 57): Spider Climb 15, Web Walker 5, Ice Walk 4, Standing Leap 3, Earth Glide 2, Tunneler 2, and body-shape traits (Amorphous, Compression, Swarm, Air Form, Water Form). Only 30 percent of these have any number in the text; they are flags.
- No Opportunity Attack when moving (MV5, 9): Flyby on seven monsters, Agile on Deer and Rat.
- Speed change on a target (MV6, 8): the dragon Slowing Breath, Scorching Sands, Freezing Burst.

### Spellcasting

90 entries on 67 monsters carry a `spellcasting` block (ability, DC, spells with per-day or at-will usage, sometimes slots): 48 are Spellcasting traits or actions, plus bonus-action spell lists such as Divine Aid, Misty Step and Teleport. 116 distinct spells appear. Another 23 entries name a spell to cast without a block (22 are dragon and lich legendary actions: "uses Spellcasting to cast Fear"). The mechanics sit in the spell data; the ability only chooses and limits the spell.

### Roll modifiers and flags

- Conditional advantage or disadvantage (AD1, 43): Pack Tactics 20, Sunlight Sensitivity 5, Blood Frenzy, Bloodied Fury, Commoner Training.
- Damage multipliers and triggered bonuses (AD2, 10): Siege Monster 4 (double damage to objects), Bloodied Fury, Bloodied Frenzy, Clay Golem Berserk.
- Breathing and environment (UT1, 47): Amphibious 31, Water Breathing 9, Hold Breath 6. No numbers in the text; flags.
- Light (UT2, 18): Illumination 5, Sunlight Sensitivity 5, Light Sensitivity.
- Entry and placement restrictions (UT4, 18 entries on 6 monsters): Vampire Spawn and the three Vampire forms (Forbiddance, Running Water, Stake to the Heart, Vampire Weakness), Fire Elemental Water Susceptibility, Flesh Golem Aversion to Fire.

### Reactions, bonus actions and legendary actions

- Reactions (24 entries, 24 monsters): 21 follow "Trigger: ... Response: ...". 11 mitigate an attack (above); the rest are Split (2), Ink Cloud (2), Pursuit, Unnerving Gaze, Shriek, Reflexive Antennae, Burst of Ingenuity, Protective Magic (3, spell-based).
- Bonus actions (89 entries, 82 monsters): Shape-Shift 22, Divine Aid 6, Nimble Escape 6, Leap 5, Swallow 4, Teleport 3, Misty Step 3, Trample 3, Charm 3, Cunning Action 2, Petrifying Gaze 2, Rampage 2, Charge 2.
- Legendary actions (87 entries, 32 monsters): Pounce 18, Frightful Presence 4, Guiding Light 4, plus a spread of one-off save effects. 44 have a cooldown, 22 cast a spell, 26 move then attack, 13 just run another of the monster's own actions (Aboleth Lash, Tail Swipe, Mummy Lord Glare). Legendary actions are the most regular group in the file.

## Abilities that fit no pattern

Two entries match no pattern at all:

- Gelatinous Cube, Transparent: a DC 15 Wisdom (Perception) check to notice the cube. It is a check with a condition on what the observer has seen.
- Will-o'-Wisp, Ephemeral: cannot wear or carry anything.

Eighty-three entries land in the "leave unmodeled" bucket because every pattern they match is in the unmodeled class (this includes the two above). They are mostly mechanical text that needs state the fight does not track, or prose the agent reads and rules on:

- Shape-Shift (25): bonus action on 22 monsters (Doppelganger, Mimic, Night Hag, the werebeasts, Vampire, and more), action on Imp, Oni and Quasit. Plus three alternate-form traits: Succubus Form, Incubus Form, Half-Dragon Draconic Origin. A stat block swap, with the equipment and hit point rules in prose.
- Revival traits (22): Diabolical Restoration 7, Demonic Restoration 5, Exalted Restoration 3, Elemental Restoration 2, and one each of Eldritch (Aboleth), Fiendish (Rakshasa), Hellish (Lemure) and Undead (Mummy Lord) Restoration, plus Lich Spirit Jar. They happen out of combat, on a timer. (25 entries match the revival pattern; Guardian Naga and Spirit Naga Restoration and Balor Death Throes also match healing or death-trigger patterns, so they count as effect-shaped.)
- Spawn and splitting (4): Black Pudding and Ochre Jelly Split, Wraith Create Specter, Troll Limb Troll Spawn.
- Senses and communication (15): Probing Telepathy, Telepathic Bond, Shark Telepathy, Sense Magic, Iron Scent, Treasure Sense, Divine Awareness (2, knows if it hears a lie), Mimicry (2), Ethereal Sight (2), Speak with Beasts and Plants, Shielded Mind, Vampiric Connection.
- Bespoke one-offs (8): Wishes (Djinni and Efreeti, a 30 percent chance of knowing Wish), Bound, Confer Fire Resistance, Freeze, Gibbering, Magic Rope, Soul Bag.
- Others (6): Rust Monster Antennae and Reflexive Antennae (metal destruction), Shrieker Fungus Shriek (a noise audible 300 feet away), Incubus Restless Touch (a curse), plus Transparent and Ephemeral.

The 265 ability names that appear on only one monster are mostly not in this list. Most are built from the same parts (a save, damage dice, a condition) and match ordinary patterns.

`--unmodeled` on the script prints the full list with monster names.

## Where the modeled line falls

This is a suggestion for the Modeled-ability vocabulary, not a decision. It is drawn from the counts above.

Effects the data supports directly, with the share of entries they cover:

- Attack roll with damage, extra damage, condition, forced move, or a save rider (AT1 to AT6): 440 attack entries.
- Saving throw with damage (half or none), conditions, forced movement, repeat save, two-stage save, immunity after success, and HP or type gates (SV1 to SV6, SV8, SV9): every save-based entry except the 15 in SV7.
- Conditions: Prone, Grappled, Restrained, Poisoned, Paralyzed, Blinded, Incapacitated, Frightened, Unconscious, Charmed, Deafened, Petrified, Stunned, Invisible. These 14 plus Exhaustion levels cover every condition the abilities inflict.
- Area template: cone, line, emanation, sphere, cube, cylinder, each with size and origin (126 entries).
- Usage: recharge (1d6, min 4, 5 or 6), per day (1, 2, 3, 4, 6), rest recharge, legendary action cooldown (88 + 61 + 1 + 44 entries).
- Multiattack: sequence of counted attacks, options, and swap clauses (188).
- Healing: regain, absorb, regenerate (19), Temporary Hit Points (9).
- Movement actions: teleport, move then attack, bonus Disengage/Hide/Dash, forced move, Speed change on target (about 90).
- Reactions that mitigate an attack: add AC, reduce damage, redirect (11).
- Spellcasting through the spell data: structured blocks (90) and spell-by-name legendary actions (23).
- Trigger and response for reactions, and Legendary Resistance and per-day counters.

Flags, as tags the engine reads and no effect to run: Magic Resistance, Pack Tactics and the other conditional advantage traits, Flyby, Amphibious and the breathing traits, Spider Climb and the terrain-traversal traits, light traits, form immunities. About 166 trait entries.

Unmodeled, as the agent's call with the text kept: Shape-Shift, revival, creating creatures or objects, persistent zones, possession, curses and long-term afflictions, senses and telepathy, and the one-off bespoke rules. 83 entries.

If the vocabulary starts smaller, the cheapest cut that keeps the large majority of the fight is attack roll, save, damage, condition, area, usage limit, multiattack and spellcasting-by-reference. Those alone account for most of the 1,080 effect-shaped entries.

## Caveats

- The tags are heuristics. Two parts are weak: the "inflicts a condition" rule reads English, and I fixed three false positives during review (negations such as "doesn't have the Incapacitated condition", prerequisites such as "Charmed or Grappled by", and "unless the azer has"). A few more misses may remain in counts under 10.
- The engine class assignment (effect, spell, flag, unmodeled) is my reading of the patterns and is in the `ENGINE` table at the top of the rules in `scripts/survey-srd-abilities.py`. Change the table and rerun to move a pattern across the line.
- Entries on form stat blocks (Vampire, werebeasts) are counted per stat block, as are dragon ages. Counts by distinct ability name are lower than the entry counts shown: 422 names for 1,392 entries.
- This survey describes the 2024 SRD as stored in this repo. Lair actions and the spells inside spell lists are outside it.
