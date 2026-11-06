# Overrides use dotted paths over the state model, and one log file records the fight

`bench override` targets state through dotted domain paths, such as `goblin-2.hp 14` or `mia.conditions '["prone"]'`, and a reason is required. Scalar values go in bare, structured values as JSON, and `--clear` removes a value (`bench override mia.concentration --clear`). The engine validates the path and value against the same schema family the stat blocks use and reports field-path errors. Every state field is a legal target except the log itself, encounter and combatant ids, hashes, and the engine version.

Adding a combatant is not an override. `bench add <statblock> --side monsters` rolls initiative, inserts the combatant at its rolled index (it acts from next round if its slot has already passed), and takes `--band near|far` or `--engaged-with <id>` for its starting position, defaulting to near its own side. An override that leaves a combatant id the encounter didn't have, such as a `combatants` list with a new entry or with an entry renamed, is refused whole with an error naming `bench add`, and so is one that leaves the same id in `combatants` twice. `bench remove <id> --reason` is the named command for departures.

Removing a combatant by override is allowed. Clearing `combatants.<id>` or `combatants.<index>`, or setting `combatants` without a creature, runs the same cleanup as `bench remove`: the creature leaves the initiative order, its rolls, and other creatures' engagements and ranges, and what it held by concentration ends first. The log records it as an OVERRIDE line, not REMOVE. Two override removals are refused whole: one that drops the creature holding the active turn (use `bench remove`, which ends the turn first when others remain), and any removal while a prompt is pending.

The log is one plain-text file per encounter, with round and turn headers and one line per event, rendered from the event lines the engine already emits. Special lines carry uppercase tags: OVERRIDE with its reason, UNDO naming the line it reverts, AGENT JUDGMENT for `bench roll` results, PROMPT with its answer including declined reaction windows, and ADD or REMOVE for `bench add` and `bench remove` (an override removal logs OVERRIDE). User-entered dice are marked inline as (player dice). House rules and standing assumptions are named wherever they change an outcome, and a batch log opens with a header block listing every standing assumption.

The public view is a filter, not a file. `bench log --public` renders the same log for players. It hides monster hit points, resources, recharge state, and per-round budgets, and it replaces hit points with wounded states. Lines that touch hidden fields keep the fact and drop the detail: an override shows its subject and reason but not its values, and a monster reaction prompt never names remaining uses. Monster AC stays visible, and the player side shows in full. The public digest uses the same rules.

## Considered options

- JSON Pointer paths into the state file. Rejected because the pointer leaks the file's JSON layout to the agent's fingers and reads badly in the log.
- One fixed subcommand per overridable field. Rejected because the agent can override nearly anything, so each new state field would force a new subcommand.
- JSON null for clearing values. Rejected because null puts JSON syntax in the agent's fingers for no gain over a flag.
- Adding combatants through `bench override`. Rejected because an add rolls initiative and chooses a starting band, which is more than setting state and deserves its own log tag.
- Refusing all override removal in favour of `bench remove`. Rejected because override already finds the dropped creatures, the cleanup is shared with `bench remove` through one function, and refusing would break agents that already clear `combatants.<id>`.
- Glyph markers in the log. Rejected because they read worse than uppercase words in plain-text editors and diffs.
- A second, redacted public log file. Rejected because two files can drift, and the reader would not know which is true.

## Consequences

- The log records the fact of every engine action, including declined reaction windows and forfeited actions, so a reader can reconstruct a fight without having to trust silence.
- Batch logs render exactly like live logs, with the standing-assumption header on top. They contain no player dice and normally no undo lines.
- Wounded-state words are fixed vocabulary (see CONTEXT.md) and cannot be overridden per encounter.
- An override can contradict an earlier log line, such as hit points set upward after a recorded hit. That is what the reason field is for, and the tamper hash stays intact because overrides go through the engine.
