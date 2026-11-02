#!/usr/bin/env python3
"""Summarize the committed baseline curve by party level and tier.

Prints Markdown tables used by docs/research/052-standard-party-at-high-levels.md:
on-CR cells per level, a tier summary, per-fixture effective CR gaps, the CR
range each level reads as Low or High, and the SRD XP-budget ratio for a
single on-CR monster.

Usage: scripts/curve-tiers.py [path/to/curve.json]
"""

import json
import statistics
import sys
from pathlib import Path

CURVE = Path(sys.argv[1] if len(sys.argv) > 1 else "data/baseline/curve.json")

TIERS = [(1, 4), (5, 10), (11, 16), (17, 20)]

XP = {1: 200, 2: 450, 3: 700, 4: 1100, 5: 1800, 6: 2300, 7: 2900, 8: 3900, 9: 5000, 10: 5900,
      11: 7200, 12: 8400, 13: 10000, 14: 11500, 15: 13000, 16: 15000, 17: 18000, 18: 20000,
      19: 22000, 20: 25000}
LOW_BUDGET = {1: 50, 2: 100, 3: 150, 4: 250, 5: 500, 6: 600, 7: 750, 8: 1000, 9: 1300, 10: 1600,
              11: 1900, 12: 2200, 13: 2600, 14: 2900, 15: 3300, 16: 3800, 17: 4500, 18: 5000,
              19: 5500, 20: 6400}
MODERATE_BUDGET = {1: 75, 2: 150, 3: 225, 4: 375, 5: 750, 6: 1000, 7: 1300, 8: 1700, 9: 2000,
                   10: 2300, 11: 2900, 12: 3700, 13: 4200, 14: 4900, 15: 5400, 16: 6100,
                   17: 7200, 18: 8700, 19: 10700, 20: 13200}


def pct(x):
    return f"{x * 100:.1f}%"


def is_low(wins, deaths, fights):
    return wins / fights >= 0.95 and deaths / fights < 0.02


def tier_of(level):
    return next(t for t in TIERS if t[0] <= level <= t[1])


def main():
    curve = json.loads(CURVE.read_text())
    fixtures = curve["fixtures"]
    cells = {(c["cr"], c["level"]): c for c in curve["cells"]}

    def stalemates(crs, level):
        rows = [cell for f in fixtures if f["cr"] in crs for cell in f["cells"] if cell["level"] == level]
        return sum(r["stalemates"] for r in rows)

    def ignored_share(crs):
        members = [f for f in fixtures if f["cr"] in crs]
        return sum(1 for f in members if f.get("ignored_abilities")) / len(members)

    print("## On-CR cells by party level\n")
    print("| Level | Fixtures | Pooled CRs | Win rate | Death rate | Stalemates | Band | Rounds median | With left-out abilities |")
    print("| --- | --- | --- | --- | --- | --- | --- | --- | --- |")
    diag = {}
    for level in range(1, 21):
        c = cells.get((float(level), level))
        if c is None:
            print(f"| {level} | 0 | none | | | | | | |")
            continue
        diag[level] = c
        crs = c["pooled_crs"]
        thin = " (thin)" if c["thin"] else ""
        print(f"| {level} | {c['fixtures']}{thin} | {', '.join(f'{x:g}' for x in crs)} | {pct(c['win_rate'])} | "
              f"{pct(c['death_rate'])} | {stalemates(crs, level)} | {c['band']} | {c['rounds']['median']:g} | "
              f"{pct(ignored_share(crs))} |")

    print("\n## Tier summary (on-CR cells, fights pooled)\n")
    print("| Tier | Levels | Fights | Win rate | Death rate | Levels reading Low | Rounds median range |")
    print("| --- | --- | --- | --- | --- | --- | --- |")
    for n, (lo, hi) in enumerate(TIERS, 1):
        rows = [diag[l] for l in range(lo, hi + 1) if l in diag]
        fights = sum(r["fights"] for r in rows)
        wins = sum(r["wins"] for r in rows)
        deaths = sum(r["deaths"] for r in rows)
        low = sum(1 for r in rows if r["band"] == "low")
        rounds = [r["rounds"]["median"] for r in rows]
        print(f"| {n} | {lo}-{hi} | {fights} | {pct(wins / fights)} | {pct(deaths / fights)} | {low} of {len(rows)} | "
              f"{min(rounds):g}-{max(rounds):g} |")

    print("\n## Per-fixture effective CR by fixture CR tier\n")
    print("Effective CR is the lowest party level where the fixture alone reads Low. "
          "Gap is effective CR minus CR; positive means the party needs more levels than the CR says. "
          "A fixture that never reads Low by level 20 counts as level 21, so its gap and any median "
          "it enters are lower bounds. \"Dragon\" means the fixture id contains \"dragon\", which "
          "includes dragon-turtle.\n")
    print("| Fixture CR | Group | Fixtures | Never Low by 20 | Median gap | Reads Low at or below its CR |")
    print("| --- | --- | --- | --- | --- | --- |")
    groups = [("all", lambda f: True), ("dragon", lambda f: "dragon" in f["id"]),
              ("non-dragon", lambda f: "dragon" not in f["id"])]
    for lo, hi in TIERS:
        for name, keep in groups:
            members = [f for f in fixtures if lo <= f["cr"] <= hi and keep(f)]
            if not members:
                continue
            gaps, never, at_or_below = [], 0, 0
            for f in members:
                eff = next((c["level"] for c in sorted(f["cells"], key=lambda c: c["level"])
                            if is_low(c["wins"], c["deaths"], c["fights"])), None)
                if eff is None:
                    never += 1
                    eff = 21
                gaps.append(eff - f["cr"])
                if eff <= f["cr"]:
                    at_or_below += 1
            median = f"{statistics.median(gaps):+g}" + (" or more" if never else "")
            print(f"| {lo}-{hi} | {name} | {len(members)} | {never} | {median} | {at_or_below} of {len(members)} |")

    print("\n## CR range by party level\n")
    print("Highest pooled CR that reads Low, and lowest pooled CR that reads High, at each level.\n")
    print("| Level | Highest Low CR | Lowest High CR |")
    print("| --- | --- | --- |")
    for level in range(1, 21):
        at = [c for c in curve["cells"] if c["level"] == level]
        low = max((c["cr"] for c in at if c["band"] == "low"), default=None)
        high = min((c["cr"] for c in at if c["band"] == "high"), default=None)
        fmt = lambda x: "none" if x is None else f"{x:g}"
        print(f"| {level} | {fmt(low)} | {fmt(high)} |")

    print("\n## SRD budget for one on-CR monster against four characters\n")
    print("| Level | CR XP | Low budget (x4) | Moderate budget (x4) | XP / Low budget |")
    print("| --- | --- | --- | --- | --- |")
    for level in range(1, 21):
        low, mod = LOW_BUDGET[level] * 4, MODERATE_BUDGET[level] * 4
        print(f"| {level} | {XP[level]:,} | {low:,} | {mod:,} | {XP[level] / low:.2f} |")


if __name__ == "__main__":
    main()
