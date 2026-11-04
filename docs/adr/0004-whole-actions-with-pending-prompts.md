# Live mode runs whole actions, and pending prompts handle decisions and dice

In live mode, one engine command resolves one declared action: every attack roll, damage roll, save and effect inside it. When the engine needs something it doesn't have, it stops and saves a pending prompt in the state file. The prompt asks for a reaction decision (Shield, an Opportunity Attack, a legendary action between turns) or for a die roll the user wants to make. Until the prompt is answered, the engine accepts only `bench respond`, `bench undo`, `bench override` and read-only commands. Then it finishes the original action.

The engine rolls all dice by default. With the encounter setting `pc_dice = ask`, a PC's rolls pause on a pending prompt and the user enters natural die faces. The engine applies modifiers, advantage and crits, rejects faces outside the die's range, and marks each roll in the log as engine-rolled or user-entered. Monster rolls always belong to the engine, so the agent can't choose a monster's outcome except through `bench override` with a reason.

## Considered options

- The agent steps through each attack, damage roll and save as separate commands. Rejected because it multiplies commands, spreads one action across many log entries, and makes the agent responsible for sequencing rules it shouldn't need to know.
- Dice passed up front on the command. Rejected because the agent can't know how many dice an action needs (advantage, crits, extra attacks) before the engine resolves it.
- The engine advances the turn on its own when action, bonus action and movement are spent. Rejected because free actions, object interactions and unmodeled abilities happen after the economy is spent.
- No pause mechanism, with reactions handled by agent judgment. Rejected because Shield and Opportunity Attacks change the outcome of an action already in progress.

## Consequences

- Every response carries `events` (each with its roll, outcome and a ready-made log line), a `digest` of the state after the command, and `pending`. The agent states no mechanical outcome that is absent from `events` or `digest`. Anything else is agent judgment, labeled as such and applied through `bench override`.
- `bench end-turn` advances turns. It runs end-of-turn and start-of-turn effects, skips combatants that are dead, unconscious or stable, rolls recharge and repeat saves, and increments the round when the order wraps. A forfeited action is logged.
- The engine opens reaction windows only for modeled reactions, Opportunity Attacks and legendary actions. The agent checks unmodeled reactions itself and applies them through an override.
  Amended by ADR 0015: an Opportunity Attack is declared on the move with `bench move --opportunity` and resolved by the engine. Until the live reaction windows ticket adds the prompt, the agent names reactors by judgment before it moves. That window will hold the `Move` request and fill its `opportunity` list.
  Amended by ADR 0016: legendary actions are declared on the end-turn with `bench end-turn --legendary` and resolve between the end of one turn and the start of the next. The engine opens no prompt for them. `digest.legendary` lists who may act, and the live reaction windows ticket may add a held end-turn prompt later.
- `bench undo` reverts one command and logs the reversal alongside the original entry. Larger corrections are overrides with a reason.
- `bench roll <expression> --reason <text>` logs rolls that fall outside the rules, such as an unmodeled ability's table. These rolls are labeled agent judgment.
- A later `bench auto-turn` that asks the batch tactics policy to play a monster's turn can call the same step function without changing this contract.
