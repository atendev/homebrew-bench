---
id: 005
title: Spell entry fields
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Build the entry fields from [Homebrew spell intake rules](../closed/002-homebrew-spell-intake-rules.md) ([ADR 0021](../../docs/adr/0021-homebrew-spell-entries-written-by-the-agent.md)): spell entries gain `casting_time`, declare their agent clauses, and accept the homebrew-only fields.

## Notes

- Skills: `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- `SpellEffect` in `bench-core/src/spell.rs` has no `casting_time`. Add it as an enum: `action`, `bonus_action`, `reaction`, `long`. Required on homebrew entries. SRD entries in `data/spells/effects.json` get it from SRD data, and `scripts/check-spell-effects.py` checks it against the SRD casting time. No entry carries it today.
- `effects.json` entries carry `agent_clauses` that `SpellEffect` does not declare, so serde drops them. Declare the field and make the schema test fail on any undeclared key, so the next mismatch shows up.
- Homebrew fields: `source: "homebrew"` and `pasted_source` (the user's text, verbatim). `pasted_source` is required when `source` is `homebrew` and refused otherwise.
- Export the entry schema through `bench schema`, so the agent can validate a homebrew file before loading it.
- Tests: every SRD entry has a `casting_time`; a homebrew entry without one fails to parse; agent clauses survive a round trip; the schema export names every field.
- Bump the engine version and add a changelog entry.

## Resolution

Spell entries require `casting_time` (`action`, `bonus_action`, `reaction`, `long`) and declare `agent_clauses`. They accept `source: "homebrew"` with `pasted_source`, required with that source and refused otherwise by `SpellEffect::parse`. `SpellEffect` denies unknown keys. All 86 SRD entries carry `casting_time`, and `check-spell-effects.py` checks it against the SRD, raising on text it can't map. `scaffold-spells.py` emits it. `bench schema spell` prints the entry schema.

Engine 0.1.36. The regenerated curve differs only in `engine_version`, so policy stays v5. Commit 7ea4cc2. The repo's only workflow runs on pull requests and tags, so no CI ran on the branch push.

Follow-ups: nested effect types still accept unknown keys, and the `pasted_source` rule should move into serde when ticket 006 builds the `--spell` loader, so the exported schema shows it. Decisions are in `.plan/reports/005-decisions.md`.
