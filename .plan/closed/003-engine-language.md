---
id: 003
title: Engine language and runtime
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

What language does the engine run in, and how does a user install it after downloading the repo?

## Notes

Upstream schemas are TypeScript, so TypeScript can import them as types. Python is the other obvious choice. The install has to work for someone who only wants to use the skill.

## Resolution

TypeScript on Bun. The upstream SRD schemas in `data/srd/2024/schemas/` are TypeScript, so the engine can use them as types directly. Users need Bun installed to run the engine.
