---
id: 021
title: License and attribution notice
type: task
mode: HITL
status: open
blocked_by: []
claimed: false
---

## Question

Pin the SRD version the project follows (5.2 or 5.2.1), then bring attribution in line with the license. `data/srd/README.md` cites "SRD 5.2" with the old wizards.com URL, but the SRD legal page requires its verbatim statement with `https://www.dndbeyond.com/srd`, an indication that the data was modified, and no other attribution to Wizards.

## Notes

Surfaced by CR and encounter guidance in SRD 5.2; its findings (`docs/research/009-cr-and-encounter-guidance.md`, license section) hold the verbatim statement and the rules. `data/srd/COMMIT` records which upstream commit the import tracks; compare it against the SRD 5.2.1 release to settle the version. The repo-side fix is small; the release-side rule (the notice must travel with shipped binaries and data, a linked notice is acceptable) lands with Packaging, SRD data and skill install. The resolution records the version chosen and what the notice says where.

## Resolution
