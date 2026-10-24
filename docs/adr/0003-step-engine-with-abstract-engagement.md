# One step function drives both modes, and position is engagement plus range bands

The engine's core is one step function that resolves a single step of an encounter. Live mode calls it once per command. `bench batch` loops it inside one call, with a built-in tactics policy choosing actions. One rules path serves both modes, so batch numbers and live play can't drift apart, and a sampled batch log reads like a live log.

The engine has no grid. Each combatant tracks which combatants it is engaged with and a coarse range band (engaged, near, far) to each opposing side. That covers reach, Opportunity Attacks, Pack Tactics and most auras without a map. A live user without a battle map can still describe it, and batch tactics stay simple.

## Considered options

- A whole-fight simulator for batch with a separate step API for live. Rejected because two rules paths would diverge.
- No position at all. Rejected because Pack Tactics, auras and Opportunity Attacks would all become agent judgment.
- A coordinate grid. Rejected for phase 1: it needs movement and targeting AI in batch and a map from the user in live mode.

## Consequences

- Movement between bands costs fixed feet (5 ft to engage or leave, 30 ft between near and far). These are our defaults, not SRD rules, so they appear in the log and a standing assumption can change them.
- In live mode the agent names area targets and the engine checks them against the bands. In batch mode the engine uses a default target count by area size, capped by the opponents engaged with or near the source.
- Adding a grid later changes the state file, the tactics policy and the live contract. Revisit this ADR first.
