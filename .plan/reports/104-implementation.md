# 104 Build batch value of modifiers on an opponent: decision report

Auto mode. Ticket type `task`, so no grilling subagent ran. One haiku auditor ran two rounds and approved on the second.

| Decision | Options | Choice | Made by | Audit |
| --- | --- | --- | --- | --- |
| How `bearer_value` keys terms by side | Branch inside the old match; a per-modifier term function | `modifier_term` returns one modifier's per-round value (or `None`), matched on target, change and side. `bearer_value` applies horizon, keep and lethargy | Main thread | Approved |
| When horizon and keep are computed | For every modifier, as before; only for a modifier that scores or carries lethargy | Only when it scores or carries lethargy. A Speed or Reactions rider (Slow, Hypnotic Pattern) no longer pays for a keep calculation. The horizon note now comes from the first scoring modifier, which carries the same tuple for one spell | Main thread | Approved |
| Safe first round on an opponent | `acts_before_opponents` as is; force `f = 0` | Forced to 0. Without it a bearer next in initiative would count as safe, and the test checks that case | Ticket notes | Approved |
| Where riders are scored | A `TimedModifier` arm in `effect_value`; the attack and save arms only | A `rider_value` call in the attack (`on_hit`) and save (`on_failure`) arms. A top-level `target` modifier in a mixed ability still scores 0, as before | Main thread | Approved |
| How riders of one attack are valued | One `bearer_value` call per rider; one call for all of them | One call, so Slow's four riders share one horizon and keep | Main thread | Approved |
| Rider nesting guard | Zero only the threat part at the limit, as the `Condition` arm reads; wrap the whole value in `nested` | Whole value in `nested`. Every rider term reads `threat` or `incoming_rolls`, and `incoming_rolls` reaches riders again from depth 0, so only the nesting counter bounds the recursion. A new test covers two sides with AC-penalty riders | Auditor raised in round 1; main thread kept the guard and added the test | Approved in round 2 |
| Opponent candidates in `modifier_plan` | All opponents; only those where the `target` modifiers score above 0 | Only those scoring above 0 on damage plus control, before the actor's own `self` modifiers are added, sorted by id after allies | Ticket notes | Approved |
| Attack Disadvantage's 5 points | Inline literal; named constant | `ATTACK_DISADVANTAGE_POINTS` (5.0), public beside the other score constants | Main thread | Approved |
| Note wording | Buff wording; ADR wording | Signed buff wording (`-2 AC against …`, `-1d4 on saves against …`). Attack terms add the threat, as the ADR's example does: `disadvantage on attack rolls against 21.1 threat`. `bonus_word` now prints the sign | Main thread | Approved |
| Version | Patch; none | Workspace 0.1.32, policy stays v5 (the ticket's "stays 4" predates ticket 111) | Ticket notes | Approved |

## Curve

Regenerated to a temporary file and diffed. Only `engine_version` changed (0.1.31 to 0.1.32). No fixture or party entry casts Bane or Slow. The stone golem's Slow is unmodeled, and the only rider on a curve input is Hypnotic Pattern's Speed 0, which scores nothing.

## Left open

- Rider value inside threat and incoming sums: a debuff rider's damage term sits in `Value.damage`, so `threat`, `incoming_rolls` and the concentration-keep DCs count it as if it were damage dealt. No curve input carries one. Added to Not yet specified.

## Gates

Skipped gates: no requirements or implementation sign-off pause (runner override). The Jev audit asked for `to-spec` and `architect`, and neither ran because ADR 0012 and the ticket already specify the design. No remote CI runs on a push (`release.yml` runs on tags and pull requests), so step 6 is skipped.
