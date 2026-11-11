---
id: 117
title: Live cast log for homebrew clauses
type: task
mode: AFK
status: open
blocked_by: [116]
claimed: false
---

## Question

Build the log rules from [Live spell casting prompts](../closed/003-live-spell-casting-prompts.md) ([ADR 0022](../../docs/adr/0022-live-spell-casts-reuse-bench-action.md)): a homebrew cast logs its agent clauses and upcast clause once it resolves.

## Notes

- Skills: `tdd`, `codebase-design`, `domain-modeling`, `code-review`.
- Order: `cast` (spell and cast level), the spell-cast window closes, then one `agent_clause` event per clause and an `upcast_clause` event when cast above its level, then saves and effect events. No new event kinds.
- A countered cast logs `spell_countered` and emits no clauses (ADR 0019 already spends the action and not the slot).
- Public view: a monster's `agent_clause` and `upcast_clause` events are hidden detail. A PC's are shown.
- A homebrew attack-roll spell honors `pc_dice = ask` as an SRD one does.
- SRD casts are unchanged. Emitting their clauses is fog on the map (SRD agent clauses on cast), because it changes every replay fingerprint.
- Tests: event order for a homebrew save spell; countered cast emits none; public view hides a monster's clauses and shows a PC's; a PC's homebrew attack spell prompts for dice under `pc_dice = ask`; an SRD cast's log is byte-identical to before.

## Resolution

<!-- Filled in at close. -->
