# The agent drives the engine through a one-shot CLI with state files

The skill has to work with any LLM agent, and the one capability every agent shares is running a shell command. So the engine is a one-shot CLI: each `bench <command>` call starts, does one thing, prints JSON, and exits. A live encounter's state lives in a JSON file that every command reads and rewrites. A long-running JSON-RPC server and an MCP server were both considered. Both keep state in memory and avoid re-reading a file per call, but a server needs process management the skill can't count on, and MCP needs an agent that supports it. Batch mode stays fast because `bench batch` plays all its fights inside one call.

## Consequences

- The binary, config and an encounter index live in `~/.bench/`, so the skill always knows where the binary is and the user can run it from any directory. Encounters and logs go to `~/.bench/` or `./.bench/` by config setting, with a per-command flag to override it.
- Since the state file sits on disk where an agent could edit it, every log entry records a hash of the state, and the engine refuses a file that doesn't match (`state_tampered`).
- Encounter files record the engine version, because users update binaries between sessions.
- An MCP wrapper over the same commands can be added later without engine changes. Switching to a stateful server would change the live mode contract and the skill, so revisit this ADR first.
