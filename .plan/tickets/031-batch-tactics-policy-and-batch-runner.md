---
id: 031
title: Batch tactics policy and batch runner
type: task
mode: AFK
status: open
blocked_by:
  - 027
  - 028
  - 030
claimed: false
---

## Question

Implement the deterministic, versioned batch tactics policy in `bench-core` and wire `bench batch` to run multi-level fight series, enforce unmodeled-ability resolution rules, reproduce individual fights by seed, and emit batch reports and sampled logs.

## Notes

Consult `implement`, `tdd`, `codebase-design`, and `domain-modeling`.

Key decisions to implement:
- **Deterministic batch tactics policy (`docs/adr/0012-deterministic-batch-tactics-policy.md`, closed ticket 016):**
  - One deterministic, versioned policy scores legal move-plus-action plans together for both monsters and PCs (`expected_damage = 1.0`, `control = 1.0`, `survival = 1.0`, `resource_cost = 0.0`, all overridable via standing assumptions).
  - Target priority evaluates next opponent action cycle threat (damage, reliable control, healing, recharge/limited-use); breaks ties by immediate expected damage, lowest current HP percentage, and stable combatant ID; excludes downed targets unless explicitly enabled.
  - Limited-use abilities are spent whenever they outscore the best at-will action; recharge abilities require rules-based recharge availability; legendary actions are spent in the current eligible window.
  - Movement uses step-engine band costs only when it enables or improves an action. Area effects use default target counts (`1` single, `2` small, `3` medium, `4`/all large or battlefield-wide), subtracting ally harm and wasted damage and picking a smaller area when net value is higher.
  - Healer revives downed allies when legal and worthwhile, and heals living allies below 50% HP only when expected benefit beats the best attack/control and the ally is likely to be downed before its next turn. Non-healers use modeled consumables below 25% HP, under lethal incoming threat, or to restore an incapacitated ally.
  - Every decision logs its reason, target, resource use, movement, and area assumptions. Policy identifier, version, and constants are hashed into baseline fingerprints.
- **`bench batch` execution and report generation (`docs/adr/0002-one-shot-cli-with-state-files.md`, `docs/adr/0005-closed-effect-vocabulary-with-agent-clauses.md`, `docs/adr/0007-chacha8-stream-seeds-and-replay.md`, closed tickets 004, 005, 012, 014, 017):**
  - Refuses homebrew stat blocks with unresolved `unmodeled` abilities or `agent_clauses` until each has an explicit standing assumption or `"ignored"` resolution; fixture stat blocks use their imported default resolutions (`neutral`, `base_form`, `skipped`, with `skipped` surfacing a prominent report warning).
  - Fights the monster against the standard party across levels 1–20 (or a requested level range / `--party <name>` resolved from `./.bench/parties/<name>/` then `~/.bench/parties/<name>/`), running 100 fights per cell by default (tunable) with a round cap (default 20, overridable).
  - Derives per-fight seeds deterministically from `--seed <master>` and fight index; `bench batch --seed X` reproduces the batch bit-for-bit, and `--fight <i>` reproduces a single fight with its full plain-text log (opening with the standing-assumptions header block).
  - Emits JSON summary on stdout plus the static, accessible HTML report and three sampled fight logs matching the structure and reading order established in `docs/prototypes/report-format.html` (closed ticket 017).

## Resolution
