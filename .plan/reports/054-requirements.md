# Ticket 054 requirements report

Ticket: `.plan/tickets/054-build-disengage-and-no-provoke-traits.md`. Depends on 053 (closed). Blocks 046.

## Requirements

- A Disengage standard action. It sets a per-turn flag on the actor. The flag clears when the actor's next turn starts, where `ActionEconomy::default()` is rebuilt in `engine.rs` (about line 1128).
- A no-provoke flag on the stat block for Flyby (7 monsters) and Agile (Deer, Rat), survey item MV5, 9 entries. The SRD importer (`scripts/import-srd-monsters.py`) emits it from the trait name. Homebrew sets it by hand.
- `opportunity_eligibility` (`engine.rs` line 975) reads both. A mover with either returns `Ineligible::Rule`, so the `opportunity_attack` notice skips it and a named reactor fails the move with the reason.
- Batch behavior does not change. The batch runner passes no `opportunity` list, `POLICY_VERSION` stays 2, and adoption stays with ticket 046.

## Acceptance criteria

- [ ] Tests first (`tdd`). A move out of engagement by a Disengaged mover emits no notice, and naming a reactor fails the whole move with a reason.
- [ ] The same holds for a Flyby or Agile mover, with no Disengage.
- [ ] The flag lasts the mover's turn only. After `end-turn` and the mover's next turn start, moves provoke again.
- [ ] Disengage costs the action. A second action is refused, as for other standard actions.
- [ ] The Disengage flag is not set by a failed or undone command. `bench undo` clears it.
- [ ] The importer sets the flag for exactly 9 monsters. Import review lists the Flyby and Agile traits as modeled flags, not neutral defaults.
- [ ] Existing 053 tests pass unchanged. Batch output for a fixed seed is byte-identical.
- [ ] `bench help --json` lists the Disengage command and flags. `bench/tests/cli.rs` `help_json_lists_every_command` and `bench/tests/skill.rs` agree with the skill text.

## Touch list

- `bench-core/src/engine.rs`: eligibility check, Disengage handling, doc comment on lines 972 to 974.
- `bench-core/src/encounter.rs` (`ActionEconomy`, or a field on `Combatant`) and `bench-core/src/stat_block.rs` (new flag; check the JSON schema output).
- `bench/src/main.rs` and `bench-core/src/live.rs`: command surface. This depends on whether Disengage is a `StepRequest` variant or an action id.
- `scripts/import-srd-monsters.py`, then regenerated `data/fixtures/*.json` (9 files), `data/srd/import-review.md` and `.json`.
- `.agents/skills/homebrew-bench/references/live-mode.md` (line 47 tells the agent to leave the flag off).
- `bench/tests/cli.rs`, `bench/tests/skill.rs`.
- `CHANGELOG.md` (Unreleased), `CONTEXT.md` (Disengage, no-provoke trait; Reactor entry near line 128).
- `docs/adr/0015-...md`: replace the last-but-two Consequences bullet, which says the function does not check a no-provoke mover. Add a consequence note, not a new ADR.
- `.plan/map.md` and the ticket Resolution, when closing.

## Risks

- Fixtures change, so the embedded fixture-set hash changes. `baseline status` and every batch report then warn the curve is stale (`baseline.rs` `stale_reasons`). Decide: regenerate `data/baseline/curve.json`, or confirm the hash covers the new field and the curve numbers are unchanged. Ticket 046 plans one regeneration at v3, so deferring may be cleaner.
- Schema change: ADR 0005 says a new effect type is a schema and version bump. A plain flag may avoid that, but check `schema_version`, `ENGINE_VERSION` and the stored-state compatibility.
- Import review drift: diff `import-review` and the 341 fixtures to confirm only the 9 change.
- Older state files lack the new `ActionEconomy` field. Use `#[serde(default)]`.
- Replay: a Disengage command must replay deterministically and undo as one unit.
- Live agents who relied on the "leave the flag off" workaround may double-handle. Update the skill text.

## Assumptions

- Disengage is a Standard Action (2024 rules: it also stops provoking for the whole turn, not only the move).
- The stat-block flag is a boolean on the block, not a new effect type, so the closed vocabulary (ADR 0005) is untouched.
- Hand-modeled homebrew can set the flag directly. Unrecognized prose stays unmodeled.
- Disengage needs no new dice and no `--pc-dice` handling.
- Counts: the survey says 9 entries on 9 monsters (Flyby on 7, Agile on 2). Verify against the SRD JSON.
- Rogue Cunning Action and Nimble Escape bonus-action Disengage (survey MV3) stay out of scope.
