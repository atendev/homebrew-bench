---
id: 003
title: Engine language and runtime
type: grilling
mode: HITL
status: closed
blocked_by: [001]
claimed: false
---

## Question

What language and runtime is the engine written in?

## Notes

The engine has to serve a batch runner and a live agent that calls it step by step, so consider how each is driven and how easily someone can download and run it.

## Resolution

Decided with the user on 2026-10-24. Recorded in `docs/adr/0001-rust-engine-shipped-as-binaries.md`.

- Language: Rust. The user wants the most speed for batch runs and knows Rust. TypeScript, Python and Go were weighed. Speed was the deciding factor, and the user was comfortable with TypeScript and Python as well.
- Distribution: prebuilt binaries per platform (Linux, macOS, Windows) from GitHub Releases, built by cargo-dist. A cargo build from source is the fallback. This replaces the user's first answer of one common runtime plus a clone. The user's condition was that building per platform stays easy. If it doesn't, reopen this.
- Structure: a Cargo workspace with a core library (rules, state, seeded randomness, log) and a thin binary on top. The library links statically into the binary, so a release is one executable with no extra files. The binary's interface is ticket 004.
- The language does not affect the agent. The agent calls the engine through the interface ticket 004 picks.
- Scripting for house rules: not decided. Lua through mlua is the leading candidate, since it vendors into the binary and sandboxes agent-written code. The house rule format ticket decides it.
- Embedding the SRD data in the binary versus shipping it as files goes to the monster schema ticket. House rules and user-added monsters are files on disk regardless.
- Speed estimates in the discussion (Rust 10 to 50 times faster than Python) were not measured.
