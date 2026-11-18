# Spell intake

Spell intake turns a pasted homebrew spell into a spell entry: one JSON file that `bench` reads from `--spell <file>`. The user reviews the entry before any fight casts it. Work the four steps in order. Each ends on a check; finish it before you move on.

## 1. Refuse or accept

Read the whole text first. When it gives no level, ask the user for one before you write anything. Write no entry when one of these holds, and give the user the message:

- **No effect in the vocabulary.** Nothing in the spell maps to an effect in `bench schema spell`: "<Name> has no effect the engine can run, so I wrote no spell entry."
- **No combat effect.** The spell does nothing in a fight, such as a ritual or a social spell: "<Name> has no combat effect, so I wrote no spell entry. In live mode I can still play it as agent judgment."
- **No level.** The text gives no level and the user doesn't confirm one when you ask: "The text gives no level for <Name> and you didn't confirm one, so I wrote no spell entry."
- **SRD id.** The id is an SRD spell id. Pick a kebab-case id from the spell's name and check it with `bench spell <id>`: `not_found` means it is free. When the name gives an SRD id, such as a homebrew Fireball, propose a distinct one, such as `fireball-variant`. If the user keeps the SRD id: "<id> is an SRD spell id, so I wrote no spell entry under it. Pick another id."

A partly modeled spell is accepted: the effects carry what the vocabulary expresses, and every other sentence becomes an agent clause.

Done when the spell is refused with its message, or accepted with an id that `bench spell <id>` reports as `not_found`.

## 2. Write the entry

Write the file to `./.bench/spells/<id>.json` unless the user names another path; `bench` reads only the path you pass it.

- Read the schema with `bench schema spell`. To copy the shape of an attack, save, area or scaling rule, print a similar SRD spell's cast card: `bench spell <srd-id>` shows its effects.
- Fill every field from the text: `id`, `name`, `level` (0 for a cantrip), `casting_time`, `concentration`, `effects`, and `scaling`, `delayed`, `agent_clauses` and `upcast_clause` when the spell has them. Set `source` to `homebrew` and put the user's text verbatim in `pasted_source`.
- `casting_time` is `action`, `bonus_action`, `reaction`, or `long` for 1 minute or more.
- Leave the caster's numbers to the engine: set every saving throw's `dc` and every attack's `modifier` to 0, and the engine adds the caster's save DC and attack bonus from its stat block's `spellcasting` block. A fixed number in the text, such as "DC 15" or "+7 to hit", stays out of the entry. Flag it. If the user keeps it at review, add it as an agent clause, never as an engine value.
- Write durations by what the text says. "1 round" and "until the start of your next turn" are `rounds: 1`, which ends when the caster starts its next turn. "Until the end of its next turn", said of the target, is `until_end_of_next_turn`. "Until the end of your next turn" on another creature has no unit: write `rounds: 1`, which runs one turn short, and flag it as inferred.
- Each sentence the effects cannot express goes in `agent_clauses`, verbatim. Never drop one.
- An upcast with a per-level amount goes in `scaling`, one rule per effect it changes (`extra_dice`, `extra_modifier`, `extra_targets`, `extra_effects`). Put upcast text that `scaling` cannot express in `upcast_clause`, verbatim.
- The engine grows a cantrip's damage dice on its own at caster levels 5, 11 and 17 (two, three and four times the dice). A cantrip that grows some other way keeps its growth sentence as an agent clause.
- A reaction spell's trigger belongs on the stat block that casts it (see `live-mode.md`). When the trigger is outside the engine's two, being hit by an attack roll and an enemy casting a leveled spell, put the trigger sentence in `agent_clauses`.

Done when every field the text supports is filled and every sentence of `pasted_source` is either in the effects or in `agent_clauses` or `upcast_clause`.

## 3. Flag

Write an intake flag, a `path` into the entry and a `note`, for each of these. None blocks.

- A value inferred rather than read: save ability, damage type, duration, target count, level, concentration, cast time.
- Wording with no standard rules reading.
- Damage outside the DMG Spell Damage table (Dungeon Master's Guide, Creating a Spell). Average damage near one of these is on the guideline; a spell with no damage on a successful save may run 25 percent higher. Name the table in the note.

  | Level | One target | Multiple targets |
  |---|---|---|
  | Cantrip | 1d10 | 1d6 |
  | 1 | 2d10 | 2d6 |
  | 2 | 3d10 | 3d6 |
  | 3 | 5d10 | 6d6 |
  | 4 | 6d10 | 7d6 |
  | 5 | 7d10 | 8d6 |
  | 6 | 10d10 | 11d6 |
  | 7 | 11d10 | 12d6 |
  | 8 | 12d10 | 13d6 |
  | 9 | 15d10 | 16d6 |

- A cantrip whose text gives no damage growth. Say the engine grows it at levels 5, 11 and 17 anyway.
- An upcast with no clear per-level amount.
- A `bonus_action` or `reaction` cast time, since the batch tactics policy may not cast the spell as written.
- A fixed DC or attack bonus left out of the entry.

A spell entry has no `intake_flags` field, and `bench` refuses an unknown key. Write the flags into `notes`, a single string, one `<path>: <note>` per line, such as `effects.0.saving_throw.ability: inferred Dexterity; the text says "dodge the flames"`.

Done when every inferred value and every item above has a flag in `notes`.

## 4. Validate and review

Run `bench spell <id> --spell <file>`. It parses the file and prints the cast card. Fix every error it reports, such as an unknown key or an SRD id, and run it again until it returns `ok`.

Then show the user a review table:

- Every field with its value and whether you read it or inferred it.
- Every agent clause and the upcast clause, verbatim.
- Every intake flag.
- The card's target line, which says how many target ids `bench action` takes.

Ask the user to confirm an inferred level by name. The user confirms, corrects in chat, or edits the file. After each change, run `bench spell` again and show the table again.

Done when the file validates, the user has confirmed the table and any inferred level, and every fixed number they chose to keep is an agent clause. Then return to the mode that will cast the spell: live mode passes the file to `bench start --spell`.
