# Deterministic batch tactics policy

Batch mode uses one deterministic tactics policy for both sides of an encounter. It evaluates legal move-plus-action plans together and chooses the highest-scoring plan. The score starts with expected damage, control value and survival value weighted 1.0 each; resource cost is 0.0 because batch measures one encounter. These named constants, ordering rules and thresholds are policy inputs that standing assumptions may override.

Target priority considers the next opponent action cycle: expected damage, reliable control, healing, and recharge or limited-use effects. Immediate expected damage breaks ties, followed by lowest current hit-point percentage and stable combatant ID. Downed combatants are not legal targets unless explicitly enabled. Limited-use abilities are spent whenever they score higher than the best at-will action; recharge abilities are available only after their rules-based recharge succeeds. Legendary actions are spent in the current eligible window when useful and never carried into a later round.

Movement is selected only as part of a plan that enables or improves an action, using the step engine's established band costs. Area effects use default target counts of 1 for single target, 2 for a small area, 3 for a medium area, and 4 or all eligible combatants for a large or battlefield-wide area. The policy includes ally harm and wasted damage in the score and uses a smaller area when its net value is better. Area counts are logged and overrideable.

The healer revives a downed ally when revival is legal and provides meaningful expected contribution before the encounter ends; otherwise revival competes normally with healing and attacks. Living allies below 50% hit points are healed only when the expected benefit exceeds the best attack or control action and the ally is likely to be downed before its next turn. Non-healers use consumables below 25% hit points, under lethal incoming threat before their next turn, or when needed to restore an incapacitated ally. Modeled variable effects use expected value; unmodeled consumables are unavailable to batch.

The policy logs its reason, target, resource use, movement and area assumptions. If a plan becomes illegal, it recomputes from the updated state; if none is legal, it takes the standard defensive or no-op action and logs that outcome. Its stable identifier and version, including policy constants, are part of baseline fingerprints so changing the policy makes prior baseline cells stale.

## Considered options

- Agent-selected batch actions: rejected because batch is one engine call and needs reproducible results without per-turn agent decisions.
- Random or resource-conserving tactics: rejected because they add variance or optimize for a future encounter that does not exist inside one batch fight.
- Separate movement and action heuristics: rejected because the best action can depend on the range band reached and the movement cost.
- Fixed area counts without logging: rejected because abstract engagement cannot recover exact geometry and the assumption must remain visible and overrideable.

## Consequences

- Baseline results are comparable across fixtures because both sides use the same named policy and version.
- Policy constants and scoring weights are part of the standing-assumption surface and must be included in logs and baseline fingerprints.
- The policy is intentionally an encounter optimizer, not a model of intelligent long-term resource management.
- More complex tactics, including multi-monster coordination and future encounter resource planning, require a new policy decision rather than silent changes to this one.
