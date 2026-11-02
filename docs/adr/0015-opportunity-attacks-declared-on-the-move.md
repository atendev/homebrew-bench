# Opportunity attacks are declared on the move

A move that breaks an engagement used to return only an `opportunity_attack` event naming who could attack. Nothing rolled the attack. In live mode the agent resolved it with `bench roll` and `bench override`, because `bench action` refuses an ordinary attack off-turn, and the batch runner ignored the event. ADR 0004 lists Opportunity Attacks among the reaction windows, and batch and live mode need one rule for them.

The engine now rolls an Opportunity Attack only when the mover's own move asks for it. `StepRequest::Move` carries an `opportunity` list of reactors, each with an optional attack action id, and live mode exposes it as `bench move <actor> <band> [--target <id>] --opportunity <reactor>[:<attack-id>]...`. The engine resolves the listed attacks in the order given, right before the band changes, because the 2024 rules say the attack happens right before the creature leaves the reactor's reach. Then it applies the move. Nothing rolls on its own. The `opportunity_attack` event remains the notice for reactors nobody named.

- **Attack.** The attack is one stat-block action whose only effect is an Attack with no range. Without an id, the reactor uses the first at-will one in stat-block order, reading Multiattack steps in place (the first Choice option first) and skipping steps that cast. A named id may be a limited-use melee action and spends its use. No stat-block field is added.
- **Eligibility.** One function decides both who the notice names and whom the resolver accepts. The reactor must be on the other side, able to act, free of a "no Reactions" modifier, and must not have used its reaction. The engine also assumes Invisible hides the mover from every reactor and that a Blinded reactor sees nothing. A sight failure is logged as an assumption. A named reactor that fails the check, or whose reach the move does not leave, fails the whole command and gives the reason.
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
- This departs from ADR 0004 until the live reaction windows ticket lands. Until then the agent names reactors by judgment before it moves, reading `digest.engagement`. The reaction window will be the same held `Move` request, and its prompt fills the `opportunity` list.
- Batch results do not change. The batch runner passes no list, and `POLICY_VERSION` stays 2. Batch adoption belongs to the ticket where the batch policy takes reactions, after Disengage and the Flyby and Agile no-provoke traits are built and read by the same eligibility function.
- The eligibility function does not yet check a no-provoke mover. Disengage and the Flyby and Agile traits are not modeled, so every move out of engagement provokes. Until the follow-up ticket adds the check to the same function, live mode relies on the agent leaving the flag off.
- Reach is still abstract. Only a broken engagement edge provokes, and reach 10 adds no extra edge.
- Forced movement, teleports and MoveThenAttack never provoke. Sentinel and reactions triggered by the attack itself are not modeled.
