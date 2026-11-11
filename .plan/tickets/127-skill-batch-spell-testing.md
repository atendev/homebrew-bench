---
id: 127
title: "Skill: batch spell testing"
type: task
mode: AFK
status: open
blocked_by: [119, 126]
claimed: false
---

## Question

Write the agent steps for testing a homebrew spell in batch ([ADR 0020](../../docs/adr/0020-spell-lift-as-the-spell-value-metric.md), [ADR 0021](../../docs/adr/0021-homebrew-spell-entries-written-by-the-agent.md)) into the `homebrew-bench` skill.

## Notes

- Skills: `writing-for-agents`, `domain-modeling`, `code-review`.
- After intake: resolve clauses through the assumptions file, run `bench batch --test-spell <id> --spell <file>`, and read the report: placement, neighbours, "level with", thin data, "never cast", "not measured", stale baseline.
- Honest output rules from `SKILL.md` apply. Never turn placement into a pass or fail.
- Point `SKILL.md` at the new steps for a pasted spell, beside the monster flow.

## Resolution

<!-- Filled in at close. -->
