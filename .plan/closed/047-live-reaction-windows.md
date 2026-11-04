---
id: 047
title: Live reaction windows
type: grilling
mode: HITL
status: closed
blocked_by: [053]
claimed: false
---

## Question

Should live mode open reaction windows when a trigger fires, so the engine prompts for a PC's reaction and names each monster that may react, instead of leaving the agent to notice triggers? Decide which of the six reaction triggers open a window, whether a window is a pending prompt that blocks the next command, and how declining one is recorded in the log and for replay.

## Notes

Today the skill tells the agent to ask the user about reactions itself (`.agents/skills/homebrew-bench/references/live-mode.md`, Reactions). Opportunity attacks follow ticket 044's answer. Pending prompts and undo follow ADR 0004. Consult `grilling` and `domain-modeling`.

Scope added by [Engine-rolled opportunity attacks](../closed/044-engine-rolled-opportunity-attacks.md): an opportunity-attack window is a held `Move` request whose prompt asks which listed reactors attack, and its answer fills the `opportunity` list. The build waits on the opportunity-attack build ticket.

Scope added by [Live legendary actions](../closed/048-live-legendary-actions.md): a legendary window could be a held `EndTurn` request whose prompt lists eligible holders and their ready actions, and its answer fills the `legendary` list. Phase 1 has no prompt, only `digest.legendary`, so decide whether this window opens alongside the reaction windows.

## Resolution

Decided on 2026-11-04 by a subagent the user delegated the decision to ("use subagents for making decisions"). It grilled itself one sub-question at a time against ADRs 0004, 0005, 0015 and 0016, tickets 007, 013, 044 and 048, the engine code and the 2024 PHB. Build ticket: [Build live reaction windows](../tickets/062-build-live-reaction-windows.md).

**Facts that shaped it.**
- No stat-block reaction carries a trigger. `ReactionTrigger` in `bench-core/src/effect.rs` is never read, and its six variants are not ADR 0005's six. The engine can't tell when any reaction applies.
- Of 23 SRD fixture reactions, only Deflect Missile has effects. The rest import as skipped: Parry (7), Protective Magic (3), Riposte and 12 others. The standard party caster has Shield, cast through `bench action` off-turn.
- Shield taken through `bench action` after a hit comes too late. The damage is already applied. PHB p. 315: +5 AC "including against the triggering attack". PHB p. 15: a reaction comes right after its trigger unless its text says otherwise.
- Dice prompts already hold a whole request and run it again on `bench respond` from the state before the command. The same dice come up because the stream is restored. `respond --decline` already exists.

**Decisions.**

1. **Which triggers open a window.** Two: a creature leaving the reactor's reach (the Opportunity Attack, from ADR 0015) and being hit by an attack roll (Shield, Parry). *Reason:* these cover every reaction the party and SRD can model. The other four ADR 0005 triggers have no modeled user, and two of them need positions or spell interruption.
2. **Where a trigger lives.** A new optional `trigger` field on a reaction entry. `ReactionTrigger` is rewritten to hold only triggers the engine opens windows for: `hit_by_attack_roll` with an `attack` filter of `any`, `melee` or `ranged`. The Opportunity Attack is not a variant, because every creature has it. *Reason:* the entry is where both Parry and a PC's Shield reaction live. The unused enum is dead code that contradicts ADR 0005.
3. **A window is a held command.** The engine meets the trigger, stops, saves a `reaction` pending prompt holding the request, and runs the request again once the prompt is answered. This works like a dice prompt. Hit windows hold whatever command made the attack: `bench action`, a move's Opportunity Attacks, or a legendary attack on `bench end-turn`. Opportunity Attack windows hold `bench move`. *Reason:* ADR 0004 and ticket 044 already say the window is the held request. Holding a request needs no mid-action engine state, and undo and replay already work for held requests.
4. **One window per reactor.** Each window offers one combatant its eligible reactions. A move that leaves three reactors opens up to three windows in initiative order, and all are asked before any attack resolves. *Reason:* CONTEXT.md already defines a reaction window as an offer to one combatant. The one decider per window is the user for a PC and the agent for a monster (ADR 0004).
5. **Who gets a window.** The Opportunity Attack window goes to each reactor that passes ADR 0015's eligibility check and is not already named in `--opportunity`. The hit window goes to the creature that was hit. It needs an unspent reaction, the ability to act, no "no Reactions" modifier, and a reaction whose trigger filter matches the attack and whose usage and spell slot are available. A critical hit still opens it, even though Shield can't turn it into a miss. One shared function lists the windows, and both live mode and batch call it. *Reason:* the window must never offer a reaction the resolver would refuse. That is the same rule ADR 0015 uses for the notice.
6. **Opportunity Attack windows always open.** `--opportunity` stays as a way to declare attacks up front. There is no up-front decline flag in phase 1. *Reason:* the ticket's point is that the engine notices triggers instead of the agent. An unwanted window costs one `bench respond --decline`, and a decline flag can come later without changing the contract.
7. **Answering.** `bench respond --take [<id>]` takes the offered reaction. The id names the reaction, or the attack for an Opportunity Attack, when more than one is on offer. `bench respond --decline` passes. A taken Opportunity Attack is added to the move's `opportunity` list and a declined one to an internal `declined` list. Hit-window answers queue in window order beside the dice faces. *Reason:* this fills the `opportunity` list as ticket 044 asked, and reuses the decline flag dice prompts already have.
8. **Resolving a taken hit reaction.** It runs right after the hit is known and before damage. It spends the reaction, its usage and any slot through the normal reaction path, with the attacker as its target. Then the hit is checked again against the new AC. A natural 20 still hits. *Reason:* this follows PHB p. 15 timing and the Shield text, and timed modifiers already land on their source.
9. **Parry is modeled.** The importer reads "is hit by a melee attack roll while holding a weapon. Response: … adds N to its AC against that attack" as `trigger: hit_by_attack_roll {attack: melee}`. Its effect is a timed AC modifier with a new duration, `triggering_attack`, that ends once that attack resolves. Any sentence after the pattern becomes an agent clause, so Riposte is partly modeled. Holding a weapon is assumed. The party generator writes the trigger on Shield from the spell's casting time. *Reason:* Parry is the most common SRD reaction, and ticket 046 needs it modeled to decide when batch takes it. Without a trigger, Shield would never open a window.
10. **Recording and replay.** The log gets `PROMPT <message>` when a window opens, then `PROMPT <message> answered: takes <name>` or `PROMPT <message> declined`, then `REACTION <reactor>: <name> (hit by <attacker>)` before the reaction's own lines. Each `respond` goes into the command history like a dice answer, so replay runs it again and undo reverts it one command at a time. The public view drops windows a monster declined and keeps reactions it took. *Reason:* the dice prompts already record answers this way. A declined monster window would show players that the monster has a reaction it held back.
11. **Legendary actions get no prompt.** ADR 0016 stands: they are declared on the end-turn and listed in `digest.legendary`. *Reason:* a legendary action doesn't change an action already in progress, and that is ADR 0004's reason to pause. A prompt would hold every end-turn in a legendary encounter, which ADR 0016 already rejected.
12. **Batch seam.** The engine asks a reaction decider at each window. In live mode, the decider reads the queued answers or asks for a prompt. In batch it is the tactics policy, and in this build the policy passes on every window, so batch numbers don't change. A passed Opportunity Attack window still emits the `opportunity_attack` notice, so batch logs read as before. *Reason:* ticket 046 decides when batch takes a reaction. It only has to replace the batch decider, with no engine changes.
13. **Version.** `ENGINE_VERSION` moves up one patch, and the baseline is not regenerated. *Reason:* a live encounter recorded before the build replays into new prompts, and the replay version warning is how users learn why. Batch numbers don't move. The fixture set (Parry) and the party (Shield trigger) fingerprints change, so the curve reads stale until ticket 056 regenerates it.

**Left out, and why.**
- The other four ADR 0005 triggers (an attack roll made against it, takes damage or becomes Bloodied, moves within N feet, makes a check or save) and the dormant spell-cast trigger. No modeled ability uses them, the within-N-feet trigger needs positions, and Counterspell is all agent clauses. The agent still handles these reactions with `bench action` off-turn or an override. A later variant can reuse the same window.
- Reactions that protect an ally (Protection) and reactions triggered by a miss. They need trigger filters that nothing modeled uses yet.
- Protective Magic, which lets the creature choose Counterspell or Shield. It stays skipped.
- Deflect Missile: it imports as modeled, but its damage reduction is dropped, and its redirect save fires without the trigger's condition. The build gives it no trigger and records the fault. It does not fix it.
- An up-front decline flag on `bench move`, and any setting that turns windows off.
- Refusing `bench action` for a reaction that has a trigger. Shield's Magic Missile trigger isn't modeled, so the agent still needs that path.
- Sentinel, Halt and Guardian, and reach 10 adding windows. These follow ticket 044.
- All batch reaction policy questions. They belong to ticket 046.
