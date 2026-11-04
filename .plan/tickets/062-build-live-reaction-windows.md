---
id: 062
title: Build live reaction windows
type: task
mode: AFK
status: open
blocked_by: [047]
claimed: true
---

## Question

Build the reaction windows decided in [Live reaction windows](../closed/047-live-reaction-windows.md). When an Opportunity Attack or a hit opens a window, live mode holds the triggering `bench move` or `bench action` on a `reaction` pending prompt, one prompt per reactor. `bench respond --take [<id>]` or `--decline` answers it, and the request runs again. The batch decider passes every window, so batch numbers stay the same.

## Notes

Consult `tdd`, `codebase-design`, `domain-modeling` and `code-review`. Read ADRs 0004, 0005, 0015 and 0016 first. Reuse the dice-prompt path in `bench-core/src/live.rs` (`run_engine`, `respond`, `stored_request`). Don't build a second way to hold a request.

Schema (`bench-core/src/effect.rs`, `stat_block.rs`):
- Replace the unused `ReactionTrigger` with one variant, `hit_by_attack_roll { attack: any | melee | ranged }`. Add `trigger: Option<ReactionTrigger>` to `AbilityEntry` with serde default and skip-if-none.
- Validate at load, beside the recharge dice check. A `trigger` outside `reactions` is refused with "a trigger belongs on a reaction".
- Add `Duration::TriggeringAttack`. It ends when the attack that opened the window has resolved. A `triggering_attack` duration on an entry without a hit trigger is refused at load.
- Every stat block that loaded before still loads. `schema_version` stays 1, by the same exception ADR 0015 recorded for `no_provoke`.

Engine (`bench-core/src/engine.rs`):
- Write one function that lists the windows a trigger opens. It is used by the Opportunity Attack path in `move_combatant` and by the hit path in `resolve_effect` (around the `hit` check). Opportunity Attack windows reuse `opportunity_eligibility` for every reactor not already in the move's `opportunity` list or its new `declined` list. Factor the shared "can react" checks (can act, no "no Reactions" modifier, reaction unspent) out of it. A hit window goes to the creature hit. It needs those same checks plus a reaction whose trigger filter matches the attack (melee means `range: None`) and whose usage and slot are available. A critical hit still opens it.
- `StepRequest::Move` gains `declined: Vec<String>`, with serde default and skip-if-empty, so old stored moves still load.
- Put a reaction decider on the state, the way `player_dice` sits on it, as `#[serde(skip)]` and `#[schemars(skip)]` so it is never stored and stays out of `game_hash`. At each window the engine asks it for take (with an id) or pass.
- Gate every window on a stored encounter setting, `settings.reaction_windows`, with a serde default of `false`. `bench start` sets it to `true`. Batch encounters leave it `false`. When it is `false`, the engine opens no window and behaves as before this build: an unnamed Opportunity Attack reactor gets only the `opportunity_attack` notice, and a hit applies its damage with no reaction. This matters because `undo` and `replay` refold the whole command history through `apply`, which ignores pending prompts. Without the gate, a stored move that left reach without `--opportunity` would stop at a window, the next command in the history would clear that prompt (`run_engine`), and the move would be lost without any error.
  - Live: answers queued from the prompt. With no answer queued, the decider signals a window, the step stops there, and the trial copy is thrown away.
  - Batch: passes every window. Ticket 046 replaces this with the tactics policy, so keep it one small, swappable piece.
  - A taken Opportunity Attack window adds to `opportunity`, a passed one to `declined`. All Opportunity Attack windows for a move are asked before any attack resolves, in initiative order.
- A taken hit reaction resolves right after the hit is known and before damage. It goes through the existing reaction path in `resolve_action`, which spends the reaction, the usage and the slot, with the attacker as the target list. Then the hit is checked again against the new AC. A natural 20 still hits. Remove `triggering_attack` modifiers once the attack resolves.
- A passed Opportunity Attack window in batch still emits the `opportunity_attack` notice, so batch logs keep their old shape.
- Prompts come in the order the engine meets them, so a PC dice prompt and a reaction window inside one request interleave correctly.

Live (`bench-core/src/live.rs`, `bench/src/main.rs`, `digest.rs`, `log.rs`):
- `PendingPrompt` with `kind: "reaction"`.
  - `actor_id` is the reactor.
  - The message names the reactor, the trigger and the options, for example `goblin-warrior: Opportunity Attack on standard-striker-03 (scimitar)?` or `standard-caster-05: hit by ogre (attack 17 vs AC 13), take shield?`.
  - Metadata holds the request, the faces so far, `engine_fills_rest`, the hit-window answers so far, and the options.
  - The prompt id is `reaction-{cursor}-{answers so far}`. Dice prompt ids (`dice-{cursor}-{n}`) can repeat when a request reruns, so they are not unique enough for reactions.
- Store `engine_fills_rest` in the metadata of both prompt kinds. Then a dice prompt the user declined, followed by a reaction window in the same command, reruns with the flag still `true` and doesn't ask for those dice again. Today `respond --decline` passes the flag only to the next `run_engine` call (around `live.rs` line 730), and nothing saves it.
- In `respond` (around `live.rs` line 693), handle `kind == "reaction"` before the branch that sends every non-dice prompt to the engine's `Respond` step. A reaction answer reruns the held request, the same way a dice answer does.
- `Command::Respond` gains `take: Option<String>` with a serde default, so the stored history from before this build still loads.
- `bench respond --take [<id>]` and `bench respond --decline` answer it. `--take` without an id works only when one option is offered. A bad id lists the options. Dice faces on a reaction prompt are refused, and `--take` on a dice prompt is refused.
- Log lines:
  - `PROMPT <message>` when the window opens.
  - `PROMPT <message> answered: takes <name>`, or `PROMPT <message> declined`.
  - `REACTION <reactor>: <name> (hit by <attacker>)` before the reaction's own lines. Opportunity Attacks keep `OPPORTUNITY ATTACK <reactor> on <mover> (reaction)`.
- The public view drops a window that a monster reactor declined, keeps one it took, and keeps every PC window.

Importer (`scripts/import-srd-monsters.py`) and party (`scripts/generate-party.py`):
- Parry pattern: "Trigger: The <name> is hit by a melee attack roll while holding a weapon. Response: The <name> adds N to its AC against that attack, possibly causing it to miss." Import it as `trigger: hit_by_attack_roll {attack: melee}` with one `timed_modifier` of AC +N and duration `triggering_attack`.
  - Any later sentence becomes an agent clause, so Riposte is partly modeled.
  - Write a review note that holding a weapon is assumed.
  - Expect 7 Parry fixtures plus the pirate captain's Riposte. Record the actual count.
- Deflect Missile on the stone giant gets no trigger. Record in the Resolution that it is mis-modeled: its damage reduction is dropped and its save fires without the trigger. Don't fix it here.
- The generator writes `trigger: hit_by_attack_roll {attack: any}` on any reaction spell whose SRD casting time reads "when you are hit by an attack roll" (Shield). Regenerate the standard party and run `--check`.

Tests (write them first):
- Engine, hit window:
  - An ogre hits the level 5 caster with an attack that Shield turns into a miss. The live action stops on a `reaction` prompt naming Shield. `--take` makes the same d20 miss and spends the reaction and the slot. `--decline` applies the same damage as today.
  - A natural 20 still hits after Shield.
  - A caster with its reaction spent, no slots, Incapacitated, or under a "no Reactions" modifier gets no window.
  - A knight's Parry opens on a melee hit and not on a ranged one. The +2 AC applies to that attack only: a second attack in the same Multiattack checks against the old AC.
- Engine, Opportunity Attack windows:
  - An untargeted move away from two goblins opens two prompts in initiative order before either attack resolves.
  - A reactor named in `--opportunity` gets no window.
  - Take one, decline the other: the log shows one attack, and the declined goblin keeps its reaction.
  - A mover downed by the first attack still stops as ADR 0015 says.
- Engine, nested windows: a goblin's Opportunity Attack hits the moving caster, which opens Shield's window inside the held move.
- Engine, end-turn: a legendary attack declared with `--legendary` that hits the caster holds the whole end-turn on Shield's window. After the answer, durations tick down once.
- Ask mode: a PC attack's dice prompt and a monster's Parry window in the same `bench action` both resolve, in order, with the user's faces kept.
- Old encounters: on a state saved before this build (no `reaction_windows` in its settings) that holds a move leaving reach without `--opportunity` and a hit on a Shield caster, `undo` and `replay` reproduce the recorded state hashes, and no window opens.
- Ask mode with a declined dice prompt: decline a PC's attack dice with `respond --decline`. The attack hits a knight and Parry's window opens. Answer it, and check that the engine fills the declined dice without asking again.
- Undo and replay: `undo` after `respond --take` brings the prompt back. `replay` rebuilds the same state hash across take, decline and nested windows. An old stored move without `declined` and an old dice prompt still load.
- Batch: a fixed-seed batch on the knight, the pirate captain and the standard party at levels 1, 5 and 11 gives the same numbers as `master`. Check this from a scratch run before committing and record it in the Resolution. If a number moves, stop and find out why.
- Load: a `trigger` on an action is refused, a `triggering_attack` duration without a trigger is refused, and every fixture and party stat block loads.

Docs:
- New ADR 0017, "Reaction windows hold the triggering command". Cover the two triggers, one window per reactor, the held request run again on respond, the decider seam, no legendary prompt, and the considered options: a mid-action suspended state, one Opportunity Attack prompt for all reactors, an up-front decline flag, and opening all six triggers.
- Amend ADR 0004: its Opportunity Attack note now points at ADR 0017, and its legendary note says the prompt was decided against in ticket 047.
- Amend ADR 0015: the departure from ADR 0004 is closed.
- Amend ADR 0005: the reaction-trigger part of the vocabulary names the one modeled trigger field, and the other triggers stay with agent judgment.
- `.agents/skills/homebrew-bench/references/live-mode.md`:
  - "Answer prompts" (around line 34): reaction prompts, `--take`, `--decline`, and that a PC's window goes to the user while a monster's goes to the agent.
  - "Reactions" (around line 54): the engine now opens windows for Opportunity Attacks and for hits. `bench action` off-turn is for the other triggers only.
  - The Opportunity Attack bullet (around line 56): the window replaces "run `bench undo` and repeat the move".
  - The skill test checks `--take` against `bench help --json`.
- `.agents/skills/homebrew-bench/references/batch-mode.md`: Parry is now modeled but the policy still takes no reactions, so those fixtures no longer list Parry as skipped.
- `CHANGELOG.md` under Unreleased:
  - the reaction prompt and `respond --take`
  - the `trigger` field and the `triggering_attack` duration
  - Parry and Riposte modeled
  - the Shield trigger on the standard caster
  - batch numbers unchanged
  - the engine version move, and why: older live encounters can replay into new prompts.

Version and baseline:
- Bump the workspace version, and with it `ENGINE_VERSION`, by one patch from what `master` holds when this lands. 059 and 061 also take the next patch in landing order. Don't regenerate `data/baseline/curve.json`.
- The engine and fixture-set stale reasons are already accepted by `stale_beyond_the_engine` in `bench-core/tests/calibration.rs` and by `batch_compares_against_the_embedded_curve_outside_the_repo` in `bench/tests/cli.rs`.
- The party reason (Shield's trigger changes the party fingerprint) is added to both by ticket 059. If this lands before 059, add it here, and 059 skips that step.
- Ticket 056 regenerates once after this lands and removes all three allowances.

Overlaps:
- `engine.rs` attack resolution is also touched by 061 (`disposition`) and 060 (lighting sources), and casting by 059.
- `live.rs` and `main.rs` are also touched by 059 (`--slot`) and 060 (`--lighting`).
- `live-mode.md`, `batch-mode.md`, `CHANGELOG.md` and ADR 0005 are also touched by 059 and 060.
- The importer is also touched by 060 (11 lighting traits), and the party regeneration by 059.
- Any order works. Whichever lands second rebases.
- [Build batch reactions policy](063-build-batch-reactions-policy.md), the build for ticket 046, waits on this one, since it replaces the batch decider and turns windows on for batch.

## Resolution
