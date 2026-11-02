# House rules

A house rule is a rule the user adds beyond the ruleset, to fill a gap or to change how a ruleset rule works. `bench` never reads its text. You apply it through overrides and rolls, and `--rule <slug>` tags each log line it changed.

## Where they live

- `~/.bench/house-rules/` holds rules that follow the user to every project.
- `./.bench/house-rules/` holds rules for one project or campaign.

Each rule is one Markdown file. Its file name without `.md` is the rule's **slug**. A project file with the same slug as a user file wins inside that project; when you list the loaded rules, tell the user which user-level rule it hides.

## Loading at session start

Read every file in both folders. Skip files with `status: disabled`. With about ten active rules or fewer, read each file in full. Past that, read each rule's title, `kind` and Applies when section, and open the full file when a rule's Applies when matches what is happening.

Tell the user which rules are active before the first command of a batch or live session.

## Applying a rule

When a rule's Applies when matches, follow its How to apply section. Pass its slug on the command that carries the change:

```
bench override goblin-2.hp 0 --reason "Massive damage: 22 damage in one hit, over half max HP" --rule massive-damage
bench roll 1d20 --reason "Massive damage system shock save" --rule massive-damage
```

`bench` rejects a slug with no matching file in either folder and names the folders it searched. Batch fights cannot apply house rules (see `batch-mode.md`).

## Writing a new rule

1. Ask the user for the rule in their own words. Ask whether it fills a gap (`kind: gap`) or changes a ruleset rule (`kind: change`), and whether it belongs to all their projects or this one.
2. Pick a short kebab-case file name. It becomes the slug in every log, so renaming the file later renames the rule in new logs only.
3. Draft the file from [house-rule-template.md](house-rule-template.md). Put the user's wording verbatim under Rule. Write Applies when as a trigger you can check during a fight, and How to apply as the exact `bench override` or `bench roll` recipe.
4. Show the user the full draft and the file path. Write it only after they confirm the wording.
5. Create the folder if it does not exist, and save the file.

Leave `engine: none` as it is. The field is reserved for scripted rules in a later version.
