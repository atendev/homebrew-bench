---
id: 044
title: Engine-rolled opportunity attacks
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

Should the engine resolve opportunity attacks itself, in batch and in live mode, and if so, how? Decide which attack a creature uses (its first melee attack, a policy pick, or a stat-block field), when the attacker's reaction is spent, how live mode lets an off-turn attack run through `bench action` instead of `bench roll` plus `bench override`, and whether a PC's opportunity attack in ask mode raises a pending prompt.

## Notes

Today a move that breaks engagement only returns an `opportunity_attack` event naming who may attack (`set_band` in `bench-core/src/engine.rs`). `bench action` refuses an ordinary attack off-turn, and the batch runner logs the event and moves on. Both batch tactics reactions and live reaction windows build on this answer. A batch change moves the baseline curve, so note whether the tactics policy version must bump. Consult `grilling` and `domain-modeling`; ADRs 0004 (pending prompts) and 0012 (tactics policy) are the constraints.

## Resolution

Drafted by a subagent playing both sides of the grilling; the user approved it on 2026-11-02 with all seven choices below accepted; the ADR 0004 note, the batch deferral line and the blocked-by wording were added afterward from an audit.

**Decision.** This ticket decides the engine mechanism and live mode. Batch adoption is deferred to ticket 046 and is not decided here. The engine rolls opportunity attacks, but only when the mover's own move asks for them. `StepRequest::Move` gains an `opportunity` list of reactors (each with an optional attack action id). The engine resolves each listed attack right before the band change, as 2024 says ("the attack occurs right before the creature leaves your reach"), then applies the move. Nothing rolls on its own. The `opportunity_attack` event stays as the notice for reactors nobody named. Live mode gets `bench move <actor> <band> [--target <id>] --opportunity <reactor>[:<attack-id>]...`. Batch adopts the mechanic in ticket 046, not here.

**1. Which attack.** 2024 allows "one melee attack with a weapon or an Unarmed Strike", never a Multiattack and not a spell. The attack is a stat-block `action` whose effects are one Attack with `range: None`. The default, when no attack id is given, is the first at-will leaf in stat-block order, walking into Multiattack steps (first Choice option first). The `Attack` effect has no weapon or spell flag, so the engine refuses a leaf that carries a cast. An explicit attack id may name any qualifying melee action, limited-use ones included, and spends its usage through the normal `resolve_action` path. No new stat-block field in phase 1: it would touch every fixture and the importer for a case homebrew has not shown. A policy pick belongs to ticket 046, which can pass the best melee leaf by expected damage (`best_single_attack` in `tactics.rs`, filtered to melee leaves).

**2. Reaction spend and reset.** `reaction_used` flips to true when the attack resolves, hit or miss. It does not flip on a declined, unnamed or refused attack, and the shared eligibility check runs first. The reset stays where it is today: `end_turn_events` clears the economy for the next active combatant, so a reaction comes back at the start of the owner's own turn, which is the 2024 rule. Eligibility is one function used by both the event and the resolver, so the notice never names an ineligible reactor. It requires: opposing side, `end_state` Active, `can_act` (no Incapacitated, Paralyzed, Petrified, Stunned, Unconscious), no active "no Reactions" modifier, reaction unused, the mover not Invisible to it and the reactor not Blinded (logged as an assumption), and the mover not flagged no-provoke (see 6). Today the event checks only `reaction_used`, and the untargeted branch of `move_combatant` iterates every combatant, allies included. A named reactor that fails the check makes the whole command fail with the reason, rather than being skipped silently. The mover's turn does not matter: the reactor acts off-turn.

**3. Live mode through `bench action`.** Rejected: letting `bench action` accept an ordinary attack off-turn, even behind a `--reaction` flag. It has no provocation to anchor to, so it would permit any off-turn attack and the log could not tie the attack to a move. The move carries the declaration instead, so one command is one logged unit, `undo` reverts the move and its attacks together, and replay needs no new command kind (the list is part of the stored arguments). An attack lands before the band changes, so a mover dropped to 0 hit points or made unable to act does not complete the move: remaining band changes are not applied and its movement feet are not charged. The agent must know before moving, which it does from `digest.engagement`. If the agent forgot, it runs `bench undo` and repeats the move with the flag. Order of resolution is the order given. Batch will use initiative order. Log lines read `OPPORTUNITY ATTACK <reactor> on <mover> (reaction)`, followed by the normal attack lines.

**4. PC opportunity attack in ask mode.** Dice only. A PC reactor named in `--opportunity` rolls through `player_roll`, so under `pc_dice = ask` `run_engine` holds the whole move request on a dice prompt exactly as it does for an action, and `respond` re-runs it. No decision prompt (should this PC react) is raised here. ADR 0004 names the Opportunity Attack as a prompt kind, and that prompt is ticket 047's reaction window: it is the same held `Move` request, but the prompt asks which listed reactors attack, and its answer fills the `opportunity` list. 044 builds the inside of that and 047 adds the pause. Until 047 lands this departs from ADR 0004, which opens reaction windows for Opportunity Attacks: the agent names the reactors by judgment before moving. That is the interim pattern ADR 0004 rejected, accepted because 047 closes it. ADR 0015 records the departure and amends ADR 0004's Opportunity Attack clause to point at 047.

**5. Batch, policy version, baseline.** 044 changes no batch result. The batch runner ignores `opportunity_attack` events today and still will, because it passes no list. Deciding that batch takes opportunity attacks is a policy decision (ADR 0012: new behavior needs a decision, not a silent change), and it is ticket 046's. Doing it there means one policy bump (v2 to v3) and one baseline regeneration (341 fixtures, 20 levels, 100 fights) instead of two. 046 must also fix an inconsistency that already exists: v2 prices an opportunity penalty when a ranged plan steps out of melee (`tactics.rs` around line 472) using `best_single_attack`, which counts ranged attacks, and the engine never delivers that damage. With real attacks, the penalty should use the same melee-leaf selection the engine uses. `POLICY_VERSION` stays 2 for 044. `ENGINE_VERSION` is the cargo package version, so the curve reads stale only when a release bumps it; the build ticket needs no bump of its own, and the report's "takes no reactions" sentence stays true until 046.

**6. Interactions.**
- Disengage: the engine has no Disengage action, so every move out of engagement provokes. Live mode is fine, because provoking is now opt-in per move (the agent leaves the flag off). Batch needs a real flag, a follow-up task (below), before 046 lets the policy take opportunity attacks.
- Flyby and Agile (survey MV5, 9 monsters): not modeled anywhere. Same follow-up ticket. They must land before batch adopts opportunity attacks, or those 9 fixtures are misread in the worse direction.
- Forced movement: unchanged. `apply_forced_move` emits no event and 2024 says being moved without using movement, action, Bonus Action or Reaction does not provoke.
- Teleport: unchanged. It clears engagement and never provokes.
- MoveThenAttack: spends feet only and changes no bands, so it opens nothing.
- Reach: still abstract. An engagement edge breaking is the only trigger; a reach-10 reactor gets no extra edge when a target steps from engaged to near. The attack uses the leaf's own reach. This is an approximation (ticket 029 said reach 10 "counts as engaged" but the engine never reads `reach` in the move path). The reactor is not required to be currently engaged in range terms when it resolves, because the attack is evaluated as of before the move.
- Untargeted move (043): the reactors are exactly the active, opposing combatants whose engagement edge the move breaks. A mover that is downed by an early attack stops the rest, so later reactors keep their reaction.
- Sentinel's Halt and Guardian, and reactions that trigger from the attack itself, are not modeled.

**Choices the user may want to overrule.**
1. Explicit `--opportunity` on the move, instead of auto-rolling every eligible one (batch-style) or a post-move `bench reaction` command. Auto-roll is simpler for the agent but takes the decision from the user in live mode and breaks ADR 0004's prompt model.
2. Batch adoption deferred to 046 so the policy bumps once. The alternative is "always attack when eligible" in 044 and bump to v3 now.
3. Default attack is the first at-will melee leaf, with no stat-block field. The alternative is a `opportunity_attack` field on the stat block (more accurate for odd homebrew, costs a schema and importer change).
4. A mover downed by an opportunity attack does not finish its move and is not charged feet. A reasonable alternative is charging the feet.
5. A named but ineligible reactor fails the whole command rather than being skipped with a warning.
6. Reach 10 gets no extra opportunity edge.
7. A limited-use melee attack is allowed only when named by id, never chosen by default.

**Proposed ADR 0015, "Opportunity attacks are declared on the move".** Outline: Context (the event today only notifies; ADR 0004 lists OAs as prompts; batch and live both need one rule). Decision (the `opportunity` list on Move, resolved before the band change; attack selection; reaction spend and reset; eligibility function). Considered options (auto-roll every eligible OA; off-turn ordinary `bench action`; a state-stored open provocation list; a stat-block field). Consequences (batch unchanged until 046; the reaction window of 047 reuses the held Move request; Disengage and no-provoke traits are a prerequisite for batch adoption).

**Follow-up tickets.**
- Build: Engine-rolled opportunity attacks (task). Shared eligibility function, `opportunity` on `StepRequest::Move`, `bench move --opportunity`, tests (reaction spent, reset on owner's turn, downed mover, ineligible reactor errors, ask-mode dice prompt, undo, replay), update `references/live-mode.md` and the `bench help --json` docs, and write ADR 0015.
- Build: Disengage and no-provoke traits (task). A Disengage standard action that sets a turn flag, and a Flyby/Agile flag read by the eligibility function, from the importer (MV5). Blocks 046's batch adoption.
- 046 gains: use the mechanic in batch (initiative order, melee-leaf pick by expected damage, decision rule), fix the v2 penalty, one v3 bump and baseline regeneration. 046 is blocked by the two build tickets, because it decides against the built behavior.
- 047 gains: the reaction window is a held Move whose prompt fills the `opportunity` list. 047 is blocked by the opportunity-attack build ticket.
