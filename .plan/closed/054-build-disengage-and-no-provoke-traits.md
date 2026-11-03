---
id: 054
title: Build Disengage and no-provoke traits
type: task
mode: AFK
status: closed
blocked_by: [053]
claimed: false
---

## Question

Add a Disengage standard action that sets a turn flag, and a Flyby and Agile no-provoke flag imported from the SRD (survey item MV5, 9 monsters), both read by the opportunity-attack eligibility function from [Engine-rolled opportunity attacks](../closed/044-engine-rolled-opportunity-attacks.md).

## Notes

Batch must not adopt opportunity attacks (ticket Batch policy takes reactions) until this lands. Uses the eligibility function from the opportunity-attack build ticket, so do that one first. Consult `tdd`, `codebase-design`, `domain-modeling`.

## Resolution

Built in commit a572d20. Requirements in [054-requirements](../reports/054-requirements.md).

- `bench disengage <actor>` (`StepRequest::Disengage`) spends the action and sets `ActionEconomy.disengaged`. The flag clears when the actor's own turn ends. It is skipped in saved state while false, so old saves and their tamper hashes still load.
- Flyby and Agile import as `Effect::NoProvoke` on nine SRD fixtures. `opportunity_eligibility` reads the Disengage flag and `StatBlock::no_provoke_ability`, which scans every ability list, so the notice and the resolver agree.
- Flyby is read like Agile and is not limited to flying out of reach, because the engine does not track movement mode.
- The schema stays at version 1. ADR 0015 records why: the variant only adds, and no loader checks `schema_version` yet. ADR 0005 notes the exception.
- Batch output is byte-identical for a fixed seed (checked against master on deer and giant-owl). The traits are still listed as skipped mechanics. `data/baseline/curve.json` was regenerated for the fixture fingerprint only.
- Audit: five haiku review rounds and no APPROVE. Every concrete problem found was fixed, and the user approved going ahead after round 5.
