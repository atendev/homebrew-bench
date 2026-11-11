---
id: 002
title: Homebrew spell intake rules
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

When a user pastes a homebrew spell, what does the agent convert, what does it flag, what does it refuse, and how does the human review the result before any run?

Decide:

- Which fields of the effects entry the agent fills from the text, and which it leaves for the human
- What counts as ambiguous wording that must be flagged, and what is refused outright
- The form the review takes, and whether the human can edit the entry or only approve it
- How an entry relates to `data/spells/effects.json`, which is keyed by SRD id

## Notes

- Grill with `grilling` and `domain-modeling`.
- Read [ADR 0005](../../docs/adr/0005-closed-effect-vocabulary-with-agent-clauses.md): homebrew abilities never get a default, and batch refuses until each has a resolution. Keep that rule.
- Read [ADR 0014](../../docs/adr/0014-spell-effects-file-authorship.md). It rejected LLM-authored entries for SRD spells because of churn. Decide how homebrew entries differ.
- Settled earlier: intake is included, with human review before any run.

## Resolution

Recorded in [ADR 0021](../../docs/adr/0021-homebrew-spell-entries-written-by-the-agent.md). The glossary gains **Spell entry** and **Intake flag**, and **Homebrew** now covers spells.

- **Relation to `effects.json`.** A homebrew entry lives in its own file, `./.bench/spells/<id>.json` by convention, with the effects-file entry schema plus `source: "homebrew"` and `pasted_source`. `effects.json` stays SRD-only and embedded. `bench` reads the file from a repeatable `--spell <file>` flag on every command that can cast, and a live encounter copies the entry into its state. A homebrew id equal to an SRD id is refused, and two loaded files with one id are an error.
- **What the agent fills.** All of it: level, `casting_time`, concentration, effects, scaling, upcast clause, agent clauses. ADR 0014's reason for rejecting agent-written entries was churn against upstream SRD text, and homebrew has no upstream. The agent never invents what the text doesn't say: an inferred value gets an intake flag. The entry uses the caster's DC and attack bonus. A fixed number in the text is flagged and left out, or kept as an agent clause if the user wants it. The user supplies only two things: confirmation of an inferred level, and any fixed number they choose to keep.
- **New field.** Spell entries gain `casting_time` (`action`, `bonus_action`, `reaction`, `long`), required on homebrew and filled from SRD data for SRD entries. The batch swap uses the matching slot, and `long` reads "not measured".
- **Flagged, never blocking.** Inferred values, wording with no standard reading, numbers off the level's norms (damage against the DMG guideline, a cantrip with no growth, an unclear upcast amount), and bonus action or reaction cast times.
- **Refused, no entry written.** No effect in the vocabulary; no combat effect (the message says live mode can still play it as agent judgment); a level the text doesn't give and the user doesn't confirm; an SRD id collision. A partly modeled spell is accepted, and each sentence outside the vocabulary becomes a verbatim agent clause, never dropped.
- **Review.** A table of every field with its value and read-or-inferred, every agent clause verbatim, every intake flag. The user can confirm, correct in chat, or edit the file. Each change is revalidated and shown again. There is no approval field: the gate is the skill step, as in monster intake.
- **Resolutions.** No default resolution, ever. Agent clauses and the upcast clause are resolved per run through the assumptions file, and batch refuses until each is resolved. The batch report fingerprints each homebrew entry.
