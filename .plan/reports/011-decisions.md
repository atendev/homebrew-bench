# Ticket 011 decisions: batch homebrew clause resolution

Branch `ticket-011-batch-homebrew-clause-resolution`. Engine 0.1.41. Audit: haiku, two rounds, APPROVE.

| Decision | Options | Choice | Made by | Audit |
|---|---|---|---|---|
| Assumption path for a clause | `spells.<id>.<n>`; `spells.<id>.clauses.<n>`; `spells.<id>.agent_clauses.<n>` plus `spells.<id>.upcast_clause` | The last: it mirrors the entry's field names, and `n` counts from 0 in file order | Main thread | Approved |
| What counts as resolved | Any key present; key present and non-blank | Non-blank. `ignored` is a valid resolution, an empty value is `invalid_argument` | Auditor raised it, main thread fixed | Approved on re-check |
| Mistyped path | Silently unused; refuse | Refuse in two layers: wrong shape in `resolve_assumptions`, a path naming no loaded clause in `prepare` | Main thread | Approved |
| Where unresolved clauses refuse | New error variant; reuse `Unresolved` | Reuse `Unresolved` with `spells.` paths. The message splits abilities from spell clauses, and `details.spell_clauses` appears only when non-empty | Main thread | Approved |
| Dotted spell ids | Split at the first dot; parse from the right | From the right, so an id with a dot still parses | Auditor raised it, main thread fixed | Approved on re-check |
| Fingerprint placement | Summary only; summary, report table and log header | All three, with `homebrew_spells` omitted from the summary when empty so output without `--spell` is unchanged | Main thread | Approved |
| Replay | Skip spell files; repeat `--spell` paths | Repeat the paths as passed, before the assumption flags | Main thread | Approved |
| Missing-spell warnings | Keep the SRD-only book; use the book with homebrew | Book with homebrew, so a loaded homebrew cast is not reported missing | Main thread | Approved (`SpellBook::srd()` is `new(&[])`) |
| Engine version | Hold; bump | 0.1.41, and the curve changes only in `engine_version`, as in earlier tickets | Main thread | Approved |

Left out: blank values on `abilities.<id>` still pass, which predates this ticket. Replay does not check a spell file's fingerprint against the report's.

Gates skipped under the runner's confirmation override: requirements and implementation notes were written by the main thread without a subagent.
