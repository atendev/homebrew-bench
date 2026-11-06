---
id: 085
title: ADR 0006 and override removal
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Graduated from Not yet specified, found by the 084 audit. [ADR 0006](../../docs/adr/0006-override-paths-and-log-format.md) says adding or removing a combatant is not an override, but override removal works and since 0.1.21 cleans up as `bench remove` does. Decide whether to amend the ADR to accept override removal or deprecate it in favour of `bench remove`, and make the docs match.

## Notes

Consult `domain-modeling` and `writing-for-agents`. Read ticket 084 and ADR 0006 first. The likely answer is to amend the ADR: override removal already shares `forget_combatant` with `bench remove`, and refusing it would break saves and agents that use it. Docs only, so no engine change, no version bump, and `POLICY_VERSION` stays 3. Check the live-mode reference and `CONTEXT.md` for wording that repeats the old claim.

## Resolution

Accepted. [ADR 0006](../../docs/adr/0006-override-paths-and-log-format.md) now says adding a combatant is not an override, while removing one by override is allowed: clearing `combatants.<id>` or `combatants.<index>`, or setting `combatants` without a creature, runs the same cleanup as `bench remove`, logs an OVERRIDE line (not REMOVE), and is refused whole for the active-turn creature or while a prompt is pending. "Refusing all override removal in favour of `bench remove`" is listed as a rejected option, because override already finds the dropped creatures, the cleanup is one shared function, and refusing would break agents that clear `combatants.<id>`. The log-tag sentence scopes ADD and REMOVE to `bench add` and `bench remove`. The live-mode reference names the pending-prompt refusal and the OVERRIDE line.

Docs only: engine stays 0.1.21, policy stays v3, no CHANGELOG entry. The audit found that a `combatants` list with a new id isn't refused by the engine; that went to Not yet specified. Audited by haiku over five rounds.
