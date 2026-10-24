# The engine is written in Rust and shipped as prebuilt binaries

Batch mode runs thousands of fights per report, and the owner wants the shortest possible run time and knows Rust, so the engine is a Rust Cargo workspace. TypeScript, Python and Go were considered. TypeScript and Python are slower, and Go was the only other compiled option but is not a language the owner knows. Downloaders get one prebuilt binary per platform from GitHub Releases, built by cargo-dist, with a cargo build from source as the fallback. This replaces the earlier "one common runtime plus a clone" install floor.

## Consequences

- The workspace has a core library for rules, state, seeded randomness and the log, plus a thin binary on top. Ticket 004 designs that binary's interface. A release is one executable per platform with no extra files.
- Scripting for house rules is undecided. Lua through mlua leads because it can be vendored and sandboxed. The house rule format decides it.
- If building binaries per platform turns out to be hard, revisit this decision.
