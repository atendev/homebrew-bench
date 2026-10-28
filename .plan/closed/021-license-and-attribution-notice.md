---
id: 021
title: License and attribution notice
type: task
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

Pin the SRD version the project follows (5.2 or 5.2.1), then bring attribution in line with the license. `data/srd/README.md` cites "SRD 5.2" with the old wizards.com URL, but the SRD legal page requires its verbatim statement with `https://www.dndbeyond.com/srd`, an indication that the data was modified, and no other attribution to Wizards.

## Notes

Surfaced by CR and encounter guidance in SRD 5.2; its findings (`docs/research/009-cr-and-encounter-guidance.md`, license section) hold the verbatim statement and the rules. `data/srd/COMMIT` records which upstream commit the import tracks; compare it against the SRD 5.2.1 release to settle the version. The repo-side fix is small; the release-side rule (the notice must travel with shipped binaries and data, a linked notice is acceptable) lands with Packaging, SRD data and skill install. The resolution records the version chosen and what the notice says where.

## Resolution

Pinned SRD 5.2.1. The pinned 5e-database commit (`data/srd/COMMIT`, a6212beb, 2026-10-02) holds 2024 data generated from the then-current SRD PDF in September 2026, after the 5.2.1 release, and the SRD legal page for 5.2.1 carries the wording Wizards currently requires. Chosen by the user over keeping 5.2.

Repo-side fix is done:

- `data/srd/README.md` now carries the verbatim SRD 5.2.1 statement with `https://www.dndbeyond.com/srd`, states that the content was modified (reformatted and restructured into JSON), and instructs that no other attribution to Wizards be included. The 5e-database MIT attribution is untouched.
- `CONTEXT.md` ruleset entry now reads "SRD 5.2.1, the 2024 edition". The Rules baseline decision's map line keeps its original 5.2 wording as history; this ticket supersedes the version detail.

Release-side rule, left to Packaging, SRD data and skill install: the notice must travel with every shipped artifact (binary, data, skill), as a `NOTICE` or `LICENSE-SRD` file in the release, with the same verbatim statement and modification note; a linked notice is acceptable under CC-BY-4.0 section 3(a)(2). Reports that quote SRD numbers carry the statement in a footer or linked notice.
