# Changelog

## Unreleased

### Added

- `bench batch <monster.json|fixture id>` runs one monster against the standard party, or `--party <name>`, at each level in `--levels` (default 1-20). It plays `--fights` fights per level (default 100) with a `--max-rounds` cap (default 20). Stdout is a JSON summary. The command writes `report.html` and three sampled fight logs to `--out`, or by default to `reports/<monster>-seed-<seed>/` under the output directory.
- `bench batch --seed <s> --fight <i>` replays one fight and prints its full log. Fight indexes run across levels, so index `i` is level `i / fights + 1`.
- Batch refuses a stat block with unmodeled abilities or agent clauses that have no SRD default resolution, and lists every offending id under `unmodeled_ability_refused`. `--assume abilities.<id>=ignored` clears one. `--assume policy.<constant>=<value>` changes a tactics policy constant, and `--assumptions <file>` reads a JSON list of `{path, value, reason}`.
- Both sides of a batch fight use tactics policy `homebrew-bench.tactics` v1. Each decision writes a `DECISION` log line with its reason, targets, resource, movement and area assumption.

### Changed

- A move now sets the range band on both combatants, not only the mover.
- The log now says when a creature falls unconscious or dies from damage.
- A condition can end when its creature takes damage. Sleep and Hypnotic Pattern now work this way.
- When a caster loses concentration, the conditions and modifiers it held by concentration end.

- With `--pc-dice ask`, `bench start` now prompts for each PC's initiative d20 before round 1, and the initiative line is marked `(player dice)`. A surprised PC enters two faces and keeps the lower. Under `--group-initiative`, PCs that share a stat block share one prompt. The encounter stays in round 0 with no active turn until `bench respond` answers.
- With `--pc-dice ask`, `bench add` of a PC stat block prompts for its initiative d20 and adds the PC once `bench respond` answers. Declining lets the engine roll it.
- A concentration save now draws its d20 from the action's own dice stream at the point its damage lands, so any dice the action rolls after that come after the save. Before this, the save reused a die the action had already drawn, and the stream lost the save's draw.
- In ask mode, `(player dice)` marks a line only when every die behind that event came from a face the user entered. PCs sharing a group initiative roll are all marked. Before this, one entered face marked every PC line the command produced, including dice the engine rolled after a decline. Events in JSON output carry `"player_dice": true` when marked. Heal, temporary hit point and death save dice are not marked yet.
- `bench override --standing` records the override as a standing assumption. Its OVERRIDE line ends in `[standing assumption]`. Every later event line whose actor or target is the assumption's subject ends in `[standing assumption: <path>]`. The subject is the combatant the path starts with, or, for `stat_blocks.<id>` paths, every combatant using that stat block. Assumptions on other state paths, such as `round` or `settings`, are recorded but tag no event lines. Any later override on the same path replaces the assumption, or drops it when the later override is not `--standing`.

### Replay compatibility

- Encounters recorded before the batch runner can diverge under `bench replay` at their first move, first drop to 0 HP, first damage to a sleeping creature, or first lost concentration, because each of these now changes the state differently.
- Encounters recorded before this change can diverge under `bench replay` from the first concentration save onward, because later dice come from different stream positions. Ask-mode encounters that used `bench add` for a PC diverge at that add, which now raises a prompt instead of rolling. Their starting initiative does not diverge, because their initial snapshot already holds it. Ask-mode encounters recorded after this change start from a snapshot taken before initiative, and replay rolls initiative again from the recorded `bench respond`.
