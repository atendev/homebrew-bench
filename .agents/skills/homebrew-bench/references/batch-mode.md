# Batch mode

Batch mode turns pasted homebrew into a stat block, settles every ability the engine cannot run, fights the monster against a party at each level, and reports the numbers beside the CR baseline curve. Work the four steps in order. Each ends on a check; finish it before you move on.

## 1. Intake

Convert the pasted text into one stat block JSON file.

- Read the schema with `bench schema monster`. To copy the shape of an attack, save, or multiattack, open a similar SRD fixture: `bench fixture list`, then `bench fixture show <id>`.
- Leave `default_resolution` off every homebrew ability, and delete it from anything you copy out of a fixture. It holds the SRD import's default. With it set, `bench batch` runs without refusing the ability (it reports a `skipped` finding, or an SRD default for `neutral` and `base_form`), so the user never gets to rule on it in step 2.
- Set `source` to `homebrew` and put the user's text verbatim in `pasted_source`. Give the block a kebab-case `id`, and give each ability a kebab-case `id` unique within the block.
- Classify every trait, action, bonus action, reaction and legendary action by `kind`:
  - `modeled` when the closed effect vocabulary in the schema expresses all of it.
  - `partly_modeled` when effects cover part of it. Put each sentence the effects cannot express in `agent_clauses`, verbatim.
  - `inert` when it has no combat effect, such as a language trait.
  - `unmodeled` when no effect fits. Keep the text in `desc` and leave `effects` empty.
- Add an `intake_flags` entry (`path` plus `note`) for every ambiguous or non-standard phrase, and for every number that disagrees with the rest of the block: a to-hit that does not match the ability modifier plus proficiency bonus, average damage that does not match its dice, a save DC off 8 + modifier + proficiency bonus, a recharge with no die range.
- Save the file where the user wants it. Unless they say otherwise, use `./.bench/monsters/<id>.json`; this is a convention of this skill, and `bench` reads only the path you pass it. Check the file with `bench schema validate <file>` until it passes.

Done when the file validates and you have shown the user the classification table (ability, kind, agent clauses) and every intake flag, and they have confirmed or corrected them.

## 2. Assumptions

`bench batch` refuses a monster while any `unmodeled` ability or any ability with `agent_clauses` lacks a resolution. Walk each one with the user and record one of these:

- **Model it.** If the user's reading fits the effect vocabulary, rewrite the ability as effects, set its `kind` to `modeled`, empty its `agent_clauses`, and revalidate. `bench batch` decides what to refuse from `kind` and `agent_clauses`, so an ability left `unmodeled` stays refused even with effects. Only effects change the numbers.
- **Ignore it.** Record `abilities.<id>` with the value `ignored`.
- **Assume an outcome.** Record `abilities.<id>` with a short value naming the reading, such as `"once per fight, no extra damage"`.

The engine never reads an assumption's value. It only stops the refusal and puts the value in the report. The fights still run whatever `effects` the ability has, so an `unmodeled` ability is left out entirely, and a `partly_modeled` ability runs its effects and leaves out only its `agent_clauses`. Tell the user exactly which text the numbers leave out.

Write the list to an assumptions file, one object per assumption:

```json
[
  { "path": "abilities.swallow", "value": "ignored", "reason": "User: swallow rarely fires before the fight ends" }
]
```

Pass it with `--assumptions <file>`. A one-off can use `--assume abilities.<id>=ignored --assume-reason "<why>"` instead. `policy.<constant>` paths tune the batch tactics policy; change them only when the user asks.

House rules do not reach batch fights: `bench batch` runs the ruleset and the tactics policy only. Tell the user which active house rules the numbers leave out.

Done when every unresolved ability id has a model, an `ignored`, or an assumed outcome the user approved.

## 3. Run

```
bench batch <file> --assumptions <file> --seed <n>
```

Defaults are levels `1-20` and 100 fights per level. Narrow with `--levels 5` or `--levels 3-8` when the user cares about a range. Below 100 fights the report warns of thin data. To test against the user's own party instead of the standard party, put one stat block per member per level (each with `level` set) in `./.bench/parties/<name>/` or `~/.bench/parties/<name>/` and pass `--party <name>`. Always pass `--seed`, or quote the seed the output echoes, so the user can rerun the batch exactly.

If the batch errors with unresolved ids, return to step 2 for those ids.

## 4. Report

Present the result from the JSON on stdout, then point to `report_path` for the full HTML report.

1. Lead with anything the numbers leave out: every `findings` entry of kind `skipped`, `ignored`, `assumed` or `agent_clause`, and every cast the sampled logs mark `(not in effects file)`. Say plainly that the results may understate or overstate the monster because of them.
2. The headline: `comparison.verdict`, the claimed CR beside `effective_cr`, and `comparison.basis`. The difficulty bands and effective CR are homebrew-bench calibration against the baseline curve, not SRD numbers. Say so.
3. A table per level: band, win rate, PC deaths, rounds (median), monster damage per round (median), stalemates, and any level `flags`.
4. Every warning in `comparison.anomalies`, `comparison.thin_cells` and `comparison.stale`, and the run's `warnings`.
5. The wording and coverage findings: each intake flag, each entry in `findings` (how the run treated every unmodeled ability, agent clause and SRD default), and each standing assumption with its reason.
6. The active house rules the numbers leave out.
7. The sampled logs in `samples`, each with its file, level and fight index.

To replay one fight with its full log, run its `replay` string from `samples` in the same working directory as the batch. It is a POSIX shell command that keeps the monster argument, seed and assumptions as you passed them.

Report what the numbers show. Give no pass or fail grade and no redesign advice unless the user asks for it. When they ask, keep advice tied to a number in the report.
