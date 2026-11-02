# Ticket 031 requirements: batch tactics policy and batch runner

Sources: ticket 031, ADR 0012, 0007, 0002, plus 0005/0006/0010 and ticket 027 resolution for cross-refs.
Repo state: `bench` CLI (bench/src/main.rs) has no `batch` subcommand; bench-core has `rng::derive_fight_seed`, `ErrorCode::UnmodeledAbilityRefused`, `DefaultResolution`, `StandingAssumption{path,value,reason}`, and no policy module. `bench party` / baseline fingerprints are ticket 019/015 territory.

## A. Tactics policy (bench-core)
1. A single policy module with stable id + integer/semver version and a constants struct; same policy for monsters and PCs.
2. Plan = (optional move) + action, enumerated as legal plans; highest score wins. Score = 1.0*expected_damage + 1.0*control + 1.0*survival + 0.0*resource_cost.
3. Weights and thresholds (50% heal, 25% consumable, area counts 1/2/3/4) are named constants overridable by standing assumptions (needs a path grammar, e.g. `policy.weights.control`).
4. Target priority ranks by next-opponent-cycle threat (damage, reliable control, healing, recharge/limited-use); ties: immediate expected damage, then lowest current HP %, then stable combatant ID. Test each tiebreak in order.
5. Downed targets are illegal unless a standing assumption enables them.
6. Limited-use ability is chosen whenever its score exceeds the best at-will action; no conserving across fights.
7. Recharge ability is legal only after a rules-based recharge roll succeeded (stream draw order fixed).
8. Legendary actions: spent in the current eligible window when useful; unused ones never carry to a later round.
9. Movement only inside a plan that enables or improves an action; uses ADR 0003 band costs; a pure-move plan never wins on its own.
10. Area effects: default target counts 1 single / 2 small / 3 medium / 4-or-all large or battlefield-wide; score subtracts ally harm and wasted damage; choose the smaller area when net value is higher; counts logged.
11. Healer revive: when legal and worthwhile (meaningful expected contribution before encounter ends); otherwise competes as a normal action.
12. Heal living ally only if ally < 50% HP AND heal benefit > best attack/control AND ally likely downed before its next turn.
13. Non-healer consumable use only when < 25% HP, lethal incoming threat before next turn, or to restore an incapacitated ally. Modeled variable effects use expected value; unmodeled consumables are unavailable.
14. Multiattack `Choice`/`Alternatives` (ADR 0005; engine currently takes first set) and optional empty steps are chosen by the policy.
15. Each decision emits a log reason, target, resource use, movement, area assumption.
16. Plan invalidated mid-turn: recompute from updated state; no legal plan: standard defensive/no-op action, logged.
17. Policy id, version, and constants feed a fingerprint hash (exposed for the baseline in ticket 015/019); changing any constant changes the hash (test).
18. Determinism: same inputs + seed yield identical decisions; scoring uses no hash-map iteration order or float nondeterminism.

## B. Batch runner CLI
19. `bench batch <monster.json|fixture id>` subcommand added; appears in `bench help --json` catalog.
20. Defaults: levels 1-20, 100 fights per cell, round cap 20.
21. Flags: level range (e.g. `--levels a-b` / `--level n`; exact name undecided), `--fights <n>`, `--max-rounds <n>`, `--party <name>`, `--seed <master>`, `--fight <i>`, output location/report path.
22. Standard party: one of 80 blocks per level (4 archetypes x 20 levels), fights are 1 monster vs 4 PCs.
23. `--party <name>` resolves `./.bench/parties/<name>/` before `~/.bench/parties/<name>/`; project wins; missing party gives `not_found`. Define the party-dir file layout (level coverage) and what happens if a custom party lacks some levels.
24. Single call plays all fights; writes no state files, per ADR 0002; stdout is JSON summary, errors use the existing error envelope.
25. Each fight ends on one side down, or round cap (counts as its own outcome, reported separately from win/loss).
26. Per-cell stats: wins/100, PC deaths (fights with >=1 death), win rate, rounds median/mean/P10-P90, monster DPR (total monster damage / rounds) median/mean/P10-P90, flags (thin data when applicable).
27. Standing assumptions accepted for batch (flag or file; undecided) and listed in the header, report and fingerprint.

## C. Seeds and reproduction
28. Master seed from `--seed`, else fresh OS entropy; always echoed in JSON, report and every log header.
29. Per-fight seed = `derive_fight_seed(master, index)`; one ChaCha8 stream per fight; no shared state across fights (parallelism-safe and order-independent).
30. `bench batch --seed X` run twice is byte-identical (JSON, HTML, logs). Add a test.
31. `--fight i` (requires `--seed`; error otherwise) replays one fight (with level selector) and prints full plain-text log whose header carries seed, fight index, level and standing-assumption block.
32. Replayed fight outcome equals that same fight inside the full batch (test).
33. Fight index unique per cell or global across levels? See ambiguities.

## D. Unmodeled-ability refusal
34. Before running, scan the monster (abilities/actions/legendary/reactions): any `kind: unmodeled` or non-empty `agent_clauses` with no `default_resolution` (homebrew) and no standing assumption / `"ignored"` resolution gives `unmodeled_ability_refused`, listing every offender id (not just the first). Zero fights run.
35. A standing assumption or `"ignored"` entry per ability id clears the refusal; define how the resolution is passed to batch (state-less, so a CLI flag or file).
36. Fixture/SRD blocks run using imported `default_resolution`: `neutral` (no effect), `base_form` (stays in stat-block form), `skipped`.
37. Every `skipped` resolution raises a prominent warning in the JSON (`warnings`) and report headline; also surface neutral/base_form/ignored in "Wording and coverage findings".
38. `partly_modeled` abilities: modeled effects run, agent clauses are treated per rules 34-37.
39. Unmodeled consumables are unavailable to the policy (rule 13). Also check party blocks (standard party must have zero).

## E. Report, HTML, logs
40. JSON stdout: schema version, engine + policy version/fingerprint, master seed, per-level cells, warnings, sample refs, paths to artifacts.
41. Static HTML matching `docs/prototypes/report-format.html` section order: banner removed; Headline (claimed CR, effective CR, claimed-CR comparison, representative band); Results by party level (all 20 rows); CR comparison and reference facts; Wording and coverage findings; Standing assumptions; Reproducibility and sampled logs; Method and limitations; footer with SRD CC-BY attribution.
42. Accessibility: `lang`, `main`, section `aria-labelledby`, table captions and `scope`, no color-only meaning, scrollable tables, no JS, self-contained or relative links only. Note the prototype reuses `id="cr"` twice; do not copy that bug.
43. Difficulty bands: Low >=95% wins and deaths <2%; Moderate >=75% and <15%; else High. Effective CR = lowest level reading Low (<1 or >20 phrasing).
44. Baseline comparison and stale-fingerprint warning depends on tickets 015/019 curve, which is not committed yet; degrade to "no baseline" warning.
45. Three sampled logs: representative, high variance, casualty/edge (hybrid rule), each linked with level, fight index, seed, and the `bench batch --seed X --fight i` command. Deterministic selection rule needed.
46. Logs: plain text per ADR 0006: header (monster, level, fight index, seed, standing-assumptions block, policy id/version), round/turn headers, policy decision lines, tags, RESULT block, skipped-mechanic warning. No player dice, usually no UNDO.
47. Written to an output dir (`--output` / config target); filenames like `<monster>-level-<n>-fight-<i>.txt`.

## F. Engine gaps from ticket 027 (prerequisites for party calibration)
48. Wire `StatBlock.spellcasting` save DC, attack bonus into `Cast` (effects-file spells carry `dc:0`, `modifier:0`).
49. Heal modifier from spellcasting ability applied to heal casts (currently 0).
50. Spend and validate `spell_slots`; honor `slot_level` and upcast in `resolve_cast`; error/illegal if no slot.
51. Cantrip dice scale at levels 5, 11, 17.
52. Bonus action execution (Second Wind, Healing Word, Mass Healing Word, Lesser Restoration) with one-per-turn limit.
53. Reaction execution (Shield, etc.), with window handling in batch where policy answers prompts without a user.
54. Hit on a PC at 0 HP adds a death-save failure (crit adds two per SRD) instead of resetting saves.
55. Replace `class.is_some()` as PC test with reading the `dying` field.
56. Regression tests per gap; 80 party blocks still validate.

## Ambiguities and open questions
- A1. Fight index scope: 0..99 per level cell, or global (level-major)? Prototype shows indexes 12/47/83 with different levels and seeds like master+index (847249 = 847201+48), which is not ChaCha-derived. Treat prototype as illustrative; `--fight i` then also needs `--level`, unless indexes are global.
- A2. Name/shape of the standing-assumption input for batch (no state file): repeatable `--assume path=value --reason`, or a JSON file? Needs a documented path grammar covering policy constants and ability resolutions.
- A3. "Ignored" resolution vs `neutral`: same effect, different provenance. Assume `ignored` is a user-supplied resolution recorded in report.
- A4. Mixed positions: starting bands, surprise and initiative for batch (per-side vs per-combatant), start range. ADR 0010/0003 may specify; check before coding.
- A5. How the policy answers pending prompts (reactions, concentration saves, PC initiative from ticket 041) with no user: engine-rolled dice only.
- A6. Round cap hit: classify as stalemate; does it count toward win rate (suggest not a win)?
- A7. Win definition for a monster fight: all monsters down vs all PCs dead vs unconscious/dying PCs.
- A8. Baseline fingerprint and CR comparison belong to ticket 015/019; how much of Headline CR cards is built here? Suggest compute bands and effective CR, mark baseline "unavailable".
- A9. Sample selection tie-breaks ("representative" = nearest median rounds? "high variance" = largest rounds or DPR deviation?).
- A10. Expected-value estimation for "likely to be downed before next turn" and "lethal incoming threat": define via the opponent's expected damage vs current HP.
- A11. Ticket status was `claimed: true` in uncommitted diff; blockers 028/030 are closed.

## Suggested minimal-but-complete scope
1. Engine gaps F (48-56) first, test-first; each lands separately.
2. `policy.rs` (A) with constants, scoring, fingerprint, decision log; unit tests per rule 4-13, 16-18.
3. `batch.rs` runner: party loader, refusal scan (D), seed derivation, fight loop, stats (B, C).
4. Log renderer reusing `log.rs` plus header and policy lines; sample selection.
5. HTML renderer from a single result struct, following the prototype; tests for section order, ids unique, scope/caption attrs.
6. CLI wiring, `--fight` reproduction, catalog entry, cli.rs integration tests (reproducible twice, `--fight` equals batch fight, refusal envelope, skipped warning).
7. Defer: custom-party layout beyond name resolution (still implement lookup), baseline curve comparison, parallelism, materializing batch fights into live state.
