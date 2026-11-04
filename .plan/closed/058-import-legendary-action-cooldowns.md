---
id: 058
title: Import legendary action cooldowns
type: task
mode: AFK
status: closed
blocked_by: []
claimed: false
---

## Question

Make the SRD importer set `usage: cooldown_until_next_turn` on legendary actions whose prose says the creature "can't take this action again until the start of its next turn".

## Notes

Found while building [Build live legendary actions](../closed/057-build-live-legendary-actions.md). 44 legendary actions across the fixtures carry that sentence, 15 of them modeled, mostly on adult and ancient dragons. All have no usage today. The engine now honors CooldownUntilNextTurn on legendary actions, but without the field batch can spend the same action up to three times a round. Change `scripts/import-srd-monsters.py`, regenerate the fixtures and the import review, and add a test that a cooldown legendary action is refused a second time in one round. The fixture fingerprint changes, and the curve then reports "the embedded fixture set changed". Leave `data/baseline/curve.json` alone; ticket 056 regenerates it once, after this lands. So that tests stay green, widen both stale allowances to accept that reason as well as the engine one: `stale_beyond_the_engine` in `bench-core/tests/calibration.rs` and the CLI test `batch_compares_against_the_embedded_curve_outside_the_repo`. Ticket 056 removes both allowances. Consult `tdd` and `domain-modeling`.

## Resolution

Built on branch `ticket-058-import-legendary-action-cooldowns`.

- The importer's `usage` step matches "The <creature> can't take this action again until the start of its next turn." and sets `usage: cooldown_until_next_turn` when the action has no structured usage, with a class (d) review note for each. A structured usage on the same action would be a class (a) mismatch, and none exists. The rule is section-agnostic, but in the SRD only legendary actions carry the sentence.
- 44 legendary actions on 29 fixtures changed, 15 of them modeled (14 adult and ancient dragons and the lich). The import review is at exit 0 with all 341 fixtures graduated. The fixture diffs add only the `usage` field.
- No new usage kind was needed. `cooldown_until_next_turn` is one of the five usage kinds in ADR 0005's closed vocabulary, and the engine already gives the rule's meaning: spending sets the action's counter to zero, and the holder's turn start resets it.
- Choice: the importer derives the value from prose instead of 44 entries in `data/srd/corrections.json`. ADR 0008 says prose never produces data and every fix is a correction, but a correction patches an upstream fault, and here there is none: 5e-database has no usage type for this rule. The match is one exact sentence, and every derivation is listed in the review, so the review is still the auditable record. Ticket 054 had already derived `no_provoke` from the Flyby/Agile sentence without recording it. ADR 0008 now has a consequence covering both exceptions, and the importer docstring names them.
- New CLI test `an_imported_cooldown_legendary_action_is_refused_a_second_time_in_one_round` uses the real lich fixture. It failed before the import change and passes after.
- `bench baseline status` now reports "the embedded fixture set changed" along with the engine reason. `stale_beyond_the_engine` and `batch_compares_against_the_embedded_curve_outside_the_repo` accept both reasons until ticket 056 regenerates the curve and removes the allowances. `data/baseline/curve.json` is unchanged.
- `cargo test --workspace` passes. Clippy shows only the five warnings already on master. `CHANGELOG.md` is updated. `CONTEXT.md` and the skill docs needed no change: `live-mode.md` already lists "is on cooldown" as a refusal.
- No new tickets.
