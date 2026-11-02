---
id: 031
title: Batch tactics policy and batch runner
type: task
mode: AFK
status: closed
blocked_by:
  - 028
  - 030
claimed: false
---

## Question

Implement the deterministic, versioned batch tactics policy in `bench-core` and wire `bench batch` to run multi-level fight series, enforce unmodeled-ability resolution rules, reproduce individual fights by seed, and emit batch reports and sampled logs.

## Notes

Consult `implement`, `tdd`, `codebase-design`, and `domain-modeling`.

Standing assumptions now exist in live mode (`bench override --standing`, see [PC initiative prompts, concentration save stream and standing assumptions](../closed/041-pc-initiative-prompts-and-concentration-stream.md)). Batch logs still need the standing-assumption header block from ADR 0006.

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
- **Engine gaps inherited from ticket 027 (see its resolution):** wire caster save DC, attack bonus and heal modifier from `StatBlock.spellcasting` into `Cast`, spend and validate `spell_slots`, scale cantrips by level, add bonus-action and reaction execution, and make a hit on a downed PC add a death-save failure. The standard party cannot be calibrated until these land.

## Resolution

`bench batch` runs one monster against a party across levels and writes `report.html` plus three sampled logs. Requirements are in `.plan/reports/031-requirements.md`.

- Engine gaps from ticket 027 are closed. Casts take save DC, attack bonus and heal modifier from `StatBlock.spellcasting`. Spell slots are spent and checked. Cantrips scale at levels 5, 11 and 17. Bonus actions and reactions execute with one use per turn. A hit on a downed PC adds a death-save failure, two on a crit. Death rules read `StatBlock.dying` instead of `class`.
- Other engine changes found while running fights. `move_combatant` sets the band on both combatants. Damage and death now log `falls unconscious` and `dies`. A condition can carry `ends_on_damage`, which Sleep and Hypnotic Pattern now set in `data/spells/effects.json`. Breaking concentration removes the conditions and modifiers that caster held by concentration. Without the last two, a level 1 Sleep held an adult red dragon down for the whole fight.
- `bench-core/src/tactics.rs` is policy `homebrew-bench.tactics` v1. It follows ADR 0012 for targets, limited use, recharge, legendary windows, movement, areas, healing, revival, consumables and multiattack picks. Constants override through `policy.<name>` assumption paths, and `tactics::fingerprint` hashes id, version and constants. Damage past a target's HP scores as wasted. Saving-throw abilities with no area shape, such as imported breath weapons, read the shape from "each creature in a 60-foot Cone" and log that they did. Buffs, forced movement and teleports score 0 in v1, and the policy takes no reactions.
- `bench-core/src/batch.rs` runs the fights. Fight indexes are global and level-major (`(level - 1) * fights + i`), so `--fight <i>` needs no level flag. It needs the same `--fights` value, which the replay command in the report includes. Both sides start near. A fight at the round cap is a stalemate, counted apart from wins and losses. When the monster wins, downed PCs count as deaths. Batch refuses abilities that are unmodeled or have agent clauses unless they have an imported default or an `abilities.<id>` assumption. `--assume path=value` and `--assumptions <file>` take standing assumptions. Agent clauses on party blocks are left out and the report says so.
- The report follows the prototype's section order without the banner and without the duplicate `id="cr"`. The baseline comparison waits for ticket 032. Until then the headline compares effective CR with the claimed CR by party level and carries a "no baseline" warning.
- `--party <name>` reads a folder of stat blocks, each with `level`, from `./.bench/parties/<name>/` then `~/.bench/parties/<name>/`. Any level the run asks for that the folder lacks fails with `not_found`.
- Spot checks at seed 7: the ogre (CR 2) reads effective CR 2. The adult red dragon (CR 17, 30 fights per level) reads 18, with legendary resistance and its legendary actions skipped and reported.

