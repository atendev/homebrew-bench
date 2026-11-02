# Live mode

You are the DM for one encounter. The user controls the party and declares each PC's action. You choose every monster action. `bench` resolves each declared action and holds the state. The honest-output rules in `SKILL.md` apply to every line you narrate.

## Set up

1. **Gather the combatants.** The user's PCs are stat blocks in the same schema as monsters (`bench schema monster`, with `class` and `level` set). Build them from the user's character sheets, or start from a standard party member: `bench party show --level <n>` prints all four, and you save the ones you want as files. Monsters are a homebrew file, or an SRD fixture id from `bench fixture list`. Validate each file with `bench schema validate <file>`.
2. **Agree the settings** with the user:
   - Who rolls PC dice: `--pc-dice ask` for the user's own dice, `--pc-dice engine` for engine rolls.
   - Starting positions: append `@near` or `@far` to a combatant; everyone starts `near` by default. Nobody starts engaged; use `bench move` on the first turns.
   - Surprise: `--surprised <combatant or side id>`, once per id.
   - Lighting has no engine setting. Apply sunlight or darkness effects as agent judgment.
3. **Start it.**

   ```
   bench start pc-a.json pc-b.json monsters=wyvern-hatchling.json@far --pc-dice ask --seed <n>
   ```

   The side defaults to `party` for stat blocks with a `class` and `monsters` otherwise; prefix `party=` or `monsters=` to set it.
4. **Roll PC initiative.** In ask mode, `bench start` returns `ok` with `pending` set to a d20 prompt for each PC's initiative, and the encounter stays at round 0. Ask the user for each face and answer with `bench respond <face>`, one prompt at a time, until `pending` is empty.

Done when `pending` is empty, `digest.active_turn` names a combatant, and you have told the user the initiative order.

## Run turns

Read `digest.active_turn` after every command. For each turn:

1. **Declare.** On a PC's turn, ask the user what the PC does. On a monster's turn, choose its action from its stat block and say what it attempts.
2. **Resolve, one command per declared action.**
   - `bench move <actor> <engaged|near|far> --target <id>` moves relative to one combatant. Without `--target` the actor moves to that band relative to every other combatant and breaks every engagement it leaves. It costs the largest single-pair change, and dead, unconscious and stable combatants add nothing to the cost. Moving to `engaged` always needs `--target`. `--feet <n>` sets the feet available for the move; it defaults to the remaining speed. `digest` lists each combatant's `ranges` and `engagement`.
   - `--opportunity <reactor>[:<attack-id>]` on a move that leaves engagement makes that reactor take its Opportunity Attack before the move. Repeat the flag for several reactors; they attack in the order given. See Opportunity attacks below.
   - `bench action <actor> <action-id> <target-id>...` resolves one action, bonus action, or reaction by its id from the stat block, every roll inside it included. Multiattack, spells and saves all go through this command.
3. **Answer prompts.** When `pending` is set, the encounter waits. `pending.message` says what it needs.
   - A PC dice prompt goes to the user. Pass their natural die faces with `bench respond <face>...`, or `bench respond --decline` to let the engine roll the rest.
   - Until the prompt is answered, only `respond`, `undo`, `override` and read-only commands run.
4. **Narrate** from `events` and `digest` only. Each event carries a ready-made `line`.
5. **End the turn** with `bench end-turn` when the actor is done. It runs end- and start-of-turn effects, recharge rolls and repeat saves, and skips combatants who are dead, unconscious or stable. Other combatants who cannot act, such as a paralyzed one, still get a turn; end it at once.

When the user asks to take a turn back, run `bench undo`; it reverts one command and restores its dice. For anything larger, use an override with a reason.

## What the engine leaves to you

Each of these is agent judgment. Label it, then record it through `bench roll <dice> --reason "<why>"` for the dice and `bench override <path> <value> --reason "<why>"` for the result.

- **Agent clauses and unmodeled abilities.** `bench action` runs only an ability's `effects`. Before you resolve one, check its `kind` and `agent_clauses` in the stat block, and apply any text the effects leave out after the command returns. A spell missing from the effects file comes back as an `unmodeled_cast` event; resolve that spell yourself.
- **Reactions.** The engine opens no reaction windows. When a trigger happens, such as a spell being cast, ask the user whether a PC reacts and decide for monsters. A reaction listed on the stat block runs with `bench action <actor> <reaction-id> <target-id>` out of turn.
- **Opportunity attacks.** The engine rolls them, but you say who takes them, before the move. Check `digest.engagement` for every creature the move leaves. Ask the user about each PC, decide for each monster, and pass every reactor on the move with `bench move <actor> <band> --opportunity <reactor>`. The reactor uses its first at-will melee attack. Append `:<attack-id>` to name a different melee attack, such as a limited-use one. The attacks resolve right before the move. If one leaves the mover unable to act, the move stops (a `move_stopped` event) and no movement is spent.
  The engine refuses the whole move when a named reactor can't react: it is on the mover's side, can't act, can't take reactions, has already used its reaction, can't see the mover (the mover is Invisible or the reactor is Blinded), or the move doesn't leave its reach. The error gives the reason. A provoking move without the flag returns an `opportunity_attack` event for each creature that could still react. If you meant one of them to attack, run `bench undo` and repeat the move with the flag. With `pc_dice = ask`, a PC reactor's dice pause the move on a dice prompt like any PC roll. The reaction comes back at the start of the reactor's own turn.
- **Legendary actions.** Live mode has no command for them. Resolve them between turns with `bench roll` and `bench override`.
- **Lighting, terrain, cover**, and every ruling the ruleset leaves open.

Override paths are dotted over the state, such as `goblin-2.hp 14` or `mia.conditions '["prone"]'`. Use `--clear` to remove a value, `--standing` when the call should hold for the rest of the fight, and `--rule <slug>` when a house rule drove it. Add or remove combatants with `bench add <file or fixture id> --side monsters --band near` and `bench remove <id> --reason "<why>"`.

## Showing players

`bench state --public` and `bench log --public` hide monster hit points, resources, recharge and damage amounts and show wounded states instead. Use the public versions for anything the user should see as a player, and the full versions for your own DM decisions.

## End

A live encounter runs until you end it. When one side is dead, fled or surrendered, or the user calls it, run `bench end --reason "<why>"`. Offer the user `bench log` for the full record and `bench replay` to recheck every state hash. `bench list` shows earlier encounters. Every live command after `bench start` takes `--encounter <id>`, placed after the command name, to target one of them; without it the command acts on the most recently started encounter that is still running.
