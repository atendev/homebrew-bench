# Changelog

## Unreleased

### Changed

- With `--pc-dice ask`, `bench start` now prompts for each PC's initiative d20 before round 1, and the initiative line is marked `(player dice)`. A surprised PC enters two faces and keeps the lower. Under `--group-initiative`, PCs that share a stat block share one prompt. The encounter stays in round 0 with no active turn until `bench respond` answers.
- With `--pc-dice ask`, `bench add` of a PC stat block prompts for its initiative d20 and adds the PC once `bench respond` answers. Declining lets the engine roll it.
- A concentration save now draws its d20 from the action's own dice stream at the point its damage lands, so any dice the action rolls after that come after the save. Before this, the save reused a die the action had already drawn, and the stream lost the save's draw.
- In ask mode, `(player dice)` marks a line only when every die behind that event came from a face the user entered. PCs sharing a group initiative roll are all marked. Before this, one entered face marked every PC line the command produced, including dice the engine rolled after a decline. Events in JSON output carry `"player_dice": true` when marked. Heal, temporary hit point and death save dice are not marked yet.
- `bench override --standing` records the override as a standing assumption. Its OVERRIDE line ends in `[standing assumption]`. Every later event line whose actor or target is the assumption's subject ends in `[standing assumption: <path>]`. The subject is the combatant the path starts with, or, for `stat_blocks.<id>` paths, every combatant using that stat block. Assumptions on other state paths, such as `round` or `settings`, are recorded but tag no event lines. Any later override on the same path replaces the assumption, or drops it when the later override is not `--standing`.

### Replay compatibility

- Encounters recorded before this change can diverge under `bench replay` from the first concentration save onward, because later dice come from different stream positions. Ask-mode encounters that used `bench add` for a PC diverge at that add, which now raises a prompt instead of rolling. Their starting initiative does not diverge, because their initial snapshot already holds it. Ask-mode encounters recorded after this change start from a snapshot taken before initiative, and replay rolls initiative again from the recorded `bench respond`.
