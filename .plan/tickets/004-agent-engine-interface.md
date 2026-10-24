---
id: 004
title: How the agent talks to the engine
type: grilling
mode: HITL
status: open
blocked_by: [001, 003]
claimed: false
---

## Question

What is the interface between the agent and the engine: CLI, library calls, MCP tools, or something else? Does one interface serve both batch and live mode?

## Notes

Live mode means the agent calls the engine many times across a conversation and reads state between calls. Batch mode means thousands of runs with no conversation.

## Resolution
