# Opportunity attacks are declared on the move

A move that breaks an engagement used to return only an `opportunity_attack` event naming who could attack. Nothing rolled the attack. In live mode the agent resolved it with `bench roll` and `bench override`, because `bench action` refuses an ordinary attack off-turn, and the batch runner ignored the event. ADR 0004 lists Opportunity Attacks among the reaction windows, and batch and live mode need one rule for them.

The engine now rolls an Opportunity Attack only when the mover's own move asks for it. `StepRequest::Move` carries an `opportunity` list of reactors, each with an optional attack action id, and live mode exposes it as `bench move <actor> <band> [--target <id>] --opportunity <reactor>[:<attack-id>]...`. The engine resolves the listed attacks in the order given, right before the band changes, because the 2024 rules say the attack happens right before the creature leaves the reactor's reach. Then it applies the move. Nothing rolls on its own. The `opportunity_attack` event remains the notice for reactors nobody named.

- **Attack.** The attack is one stat-block action whose only effect is an Attack with no range. Without an id, the reactor uses the first at-will one in stat-block order, reading Multiattack steps in place (the first Choice option first) and skipping steps that cast. A named id may be a limited-use melee action and spends its use. No stat-block field is added.
- **Eligibility.** One function decides both who the notice names and whom the resolver accepts. The reactor must be on the other side, able to act, free of a "no Reactions" modifier, and must not have used its reaction. The mover must not have taken Disengage this turn or have an ability with the `no_provoke` effect. The reactor must also see the mover under ADR 0017's sight rule: an Invisible mover is hidden from a reactor without Blindsight or Truesight, and a Blinded reactor sees only with Blindsight (amended by ticket 068). A sight failure is logged as an assumption. A named reactor that fails the check, or whose reach the move does not leave, fails the whole command and gives the reason.
- **Reaction spend and reset.** The reaction is spent when the attack resolves, hit or miss. It returns at the start of the reactor's own turn.
- **A downed mover.** If an attack leaves the mover unable to act, the move stops. No band changes, no feet are charged, and the remaining reactors keep their reactions.
- **Log.** Each attack opens with `OPPORTUNITY ATTACK <reactor> on <mover> (reaction)`, followed by the normal attack lines.

## Considered options

- **Auto-roll every eligible Opportunity Attack.** Simpler for the agent, but it takes the choice away from the user in live mode and does not fit ADR 0004's prompt model.
- **An off-turn ordinary attack through `bench action`, perhaps behind a `--reaction` flag.** Rejected. Nothing ties the attack to a provocation, so any off-turn attack would pass, and the log could not link the attack to its move.
- **An open-provocation list stored in the state, answered by a later command.** Rejected. It splits one rules event across two commands and two log entries, and undo would have to revert them together.
- **An `opportunity_attack` field on the stat block.** More accurate for unusual homebrew, but it touches every fixture and the importer for a case no homebrew has shown yet.

## Consequences

- One command is one logged unit. `bench undo` reverts the move and its attacks together, and replay needs no new command kind because the list is part of the stored arguments.
- Under `pc_dice = ask`, a PC reactor's dice hold the whole move on a dice prompt, the same way an action is held, and `bench respond` runs it again.
- This departed from ADR 0004 until the live reaction windows ticket landed. That departure is closed: ADR 0018 opens a window for each eligible reactor the move doesn't name, holding the same `Move` request, and its answer fills the `opportunity` list or the new `declined` list. `--opportunity` still declares attacks up front.
- Batch results do not change. The batch runner passes no list, and `POLICY_VERSION` stays 2. Batch adoption belongs to the ticket where the batch policy takes reactions. Amended by ticket 063: batch still passes no list, but its encounters open Opportunity Attack windows and policy v3 decides them (ADR 0012).
- A mover that took Disengage this turn (`bench disengage`, which spends the action and sets a flag until the mover's turn ends) or has a `no_provoke` trait (Flyby, Agile) provokes nothing. The same eligibility function checks both, so the notice never names a reactor the resolver would refuse. Flyby is read as Agile: it is not limited to flying out of reach, because the engine does not track the movement mode.
- `no_provoke` is a new effect type, and ADR 0005 calls for a schema version bump. The schema stays at version 1. The variant only adds to the vocabulary, so every stat block that loaded before still loads, and no loader reads `schema_version` yet. The first bump waits until a loader checks the version and can report a mismatch.
- Reach is still abstract. Only a broken engagement edge provokes, and reach 10 adds no extra edge.
- Forced movement, teleports and MoveThenAttack never provoke. Sentinel and reactions triggered by the attack itself are not modeled.
