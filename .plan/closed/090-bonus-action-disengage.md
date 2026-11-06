---
id: 090
title: Bonus-action Disengage
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

Nimble Escape (8 fixtures, goblins among them) and Cunning Action import as unmodeled and are skipped. Under policy v3 those creatures pay Opportunity Attacks they would avoid at a table, so they read a little weaker than they play. Should Nimble Escape and Cunning Action import as a bonus action that runs the Disengage step, and when should the batch policy spend it?

## Notes

- Source: [Batch policy takes reactions](../closed/046-batch-policy-takes-reactions.md).
- Open sub-questions: whether the import maps the trait text to a structured bonus action or leaves it to the agent, whether Hide and Dash options from the same traits are in scope, and the trigger for the policy (leave a band while an enemy engaged with the creature can react).
- Check the replay and curve impact: a policy change that alters monster turns means a new policy version and a regenerated curve.
- Skills: `grilling`, `domain-modeling`. If the answer is code, the build is a follow-up `task` ticket.

## Resolution

Grilled on 2026-11-06. The user answered round 1 and handed rounds 2 and 3 to a subagent, then confirmed the whole decision.

**Import.** A new closed-vocabulary effect, `disengage`, sets the same Disengage flag as `bench disengage` and is an additive variant at schema version 1, as `no_provoke` was. It goes on the bonus-action entry of the four wordings that say the creature "takes the … Disengage … action":

- Nimble Escape: goblin warrior, goblin minion, goblin boss, panther, tiger, saber-toothed tiger. The ticket's 8 was these 6 plus Cunning Action's 2.
- Cunning Action: assassin, spy.
- Deathless Agility: vampire spawn, vampire familiar.
- Hasten: clay golem. It keeps its Recharge 5–6.

Hide and Dash stay agent clauses, so each ability becomes partly modeled. Prowl (weretiger) stays unmodeled.

**Live mode.** The agent runs it as `bench action <actor> nimble-escape` with no target. The log names the ability rather than "takes the Disengage action". The live-mode reference says three things. A bonus Disengage avoids Opportunity Attacks without spending the action. When the DM picks Hide or Dash instead, an override marks the bonus action used. Hasten's Dash half stays the DM's call.

**Batch policy v4.** The policy never takes a standalone Disengage. It takes one only to step out of engagement without provoking. When the chosen plan steps out, the creature may Disengage before the move with a ready bonus Disengage, or with an extra action that allows Disengage (Haste). The cost is the lower of two figures, both in score units:

- the weighted Opportunity Attack penalty, paid by not disengaging;
- the score of the best plan the Disengage gives up.

When both kinds are ready at equal cost, the extra action goes first. The extra action has to be decided before the action's move, which is out of the current Action, Bonus, Extra order. The reason line names the ability that avoided the attacks. A teleport's "Opportunity Attacks avoided" value is capped when a free Disengage was available. Hasten's recharge counts as a resource cost. No hit-and-run.

**Versions and docs.** `POLICY_VERSION` goes to 4 and the curve is regenerated. Replay is unaffected: a saved live encounter folds from the stat blocks in its initial state, so encounters recorded before the import refold as recorded, and a v3 batch `--seed --fight` rerun matches only on a v3 binary, as with every earlier policy bump. Tickets 094 and 097 bump again later if they change the policy. ADR 0005 gains the effect. ADR 0012's "The policy never plans the Disengage action" and its extra-action sentence's "and never Disengage" both become "only to step out without provoking". Batch findings stop labelling `no_provoke` and `disengage` abilities "Not modeled in phase 1".

**What changes.** Among SRD fixtures only the assassin should change, since its light crossbow beats its shortsword. Goblins, the spy and the melee-only creatures play the same. Their ranged and melee attacks tie, or they have no ranged attack, and melee never leaves engagement. Their at-the-table edge is hit-and-run, which needs a value for distance that range bands lack. That went to the fog. The gain is correct batch play for homebrew archers with these traits, and a real Disengage in live mode.

**Glossary.** CONTEXT.md gains Bonus action, and Disengage now covers the action, bonus action and extra action forms.

**Fog.** Dash as a movement budget, Hide, Prowl and hit-and-run.

Build: [099 Build bonus-action Disengage](../tickets/099-build-bonus-action-disengage.md).
