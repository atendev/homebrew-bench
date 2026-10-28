# House rules are prose files in two layers, tagged in the log by file-name slug

A house rule is one Markdown file with a small frontmatter (`status`, `kind`, `engine`) and four sections: Rule, Applies when, How to apply, Notes. The agent writes it only after the user confirms the wording, and reads every file at the start of a session. Files live in `~/.bench/house-rules/` for rules that follow the user and in `./.bench/house-rules/` for a project, and the project layer wins on a same-named rule. The earlier plan was a gitignored folder at the repo root, but a user who installs the binary through the install script has no checkout.

The file name without `.md` is the rule's slug. When the agent applies a rule, it passes `--rule <slug>` to `bench override` or `bench roll`, and the engine writes `[house rule: slug]` into the log line. The engine checks the slug against the file names in both layers and rejects an unknown one with an error naming the folders searched. It never reads file contents. The tag is structural so a report can list which rules fired by counting log lines.

Phase 1 runs house rules as prose only. The `engine:` frontmatter field is reserved and set to `none`, which leaves room for scripted or modeled rules later without a format change. Lua through mlua stays undecided.

## Considered options

- A repo-root folder only. Rejected because the install-script path has no repo, and the other user-owned files already live under `~/.bench/` and `./.bench/`.
- A numbered id or a separate `id:` field. Rejected because the slug cannot drift from the file and reads better in a plain-text log.
- Free text in the override reason. Rejected because counting fired rules would mean parsing prose, and a typo would go unnoticed.
- A `--rule` flag with no validation. Rejected because a mistyped slug would silently drop out of the report's rule count.
- A `bench rules` listing command. Rejected for phase 1 because the engine knows nothing about house rule content, and the agent already reads files.
- Engine-enforced per-run rule switches. Rejected because the engine has no behavior to switch; the agent controls what is applied through overrides.
- Committing to Lua through mlua now. Rejected because it adds a vendored interpreter and sandbox design to a phase that never runs a script.

## Consequences

- The engine depends on the folder layout for slug validation, so moving the folders is a breaking change for old logs and for the skill.
- Renaming a rule file changes its name in later logs, and existing logs keep the old name.
- A same-named rule in both layers hides the user-level one inside that project, and the agent says so when it lists loaded rules.
- A user who wants a reproducible comparison with and without a rule relies on the agent and the report's rule list, not on an engine setting.
