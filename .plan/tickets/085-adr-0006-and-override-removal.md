---
id: 085
title: ADR 0006 and override removal
type: task
mode: AFK
status: open
blocked_by: []
claimed: true
---

## Question

Graduated from Not yet specified, found by the 084 audit. [ADR 0006](../../docs/adr/0006-override-paths-and-log-format.md) says adding or removing a combatant is not an override, but override removal works and since 0.1.21 cleans up as `bench remove` does. Decide whether to amend the ADR to accept override removal or deprecate it in favour of `bench remove`, and make the docs match.

## Notes

Consult `domain-modeling` and `writing-for-agents`. Read ticket 084 and ADR 0006 first. The likely answer is to amend the ADR: override removal already shares `forget_combatant` with `bench remove`, and refusing it would break saves and agents that use it. Docs only, so no engine change, no version bump, and `POLICY_VERSION` stays 3. Check the live-mode reference and `CONTEXT.md` for wording that repeats the old claim.
