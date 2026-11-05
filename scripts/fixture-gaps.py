#!/usr/bin/env python3
"""Compare per-fixture effective CR between two baseline curves.

Prints Markdown tables used by docs/research/064-dragons-on-the-v3-curve.md:
effective CR and gap for every fixture whose id contains one of the given
substrings, on the old and the new curve, a gap summary by fixture CR tier for
dragons and non-dragons on both curves, and wins and deaths at levels 17 to 20
for every fixture of CR 19 or more.

Effective CR is the lowest party level where the fixture alone reads Low
(at least 95% wins and deaths in under 2% of fights). A fixture that never
reads Low by level 20 counts as 21.

Usage: scripts/fixture-gaps.py OLD.json NEW.json [substring ...]
Default substrings: dragon pit-fiend lich
"""

import json
import statistics
import sys
from pathlib import Path

TIERS = [(1, 4), (5, 10), (11, 16), (17, 20)]
HIGH_LEVELS = range(17, 21)


def effective(fixture):
    for cell in sorted(fixture["cells"], key=lambda c: c["level"]):
        if cell["wins"] / cell["fights"] >= 0.95 and cell["deaths"] / cell["fights"] < 0.02:
            return cell["level"]
    return 21


def load(path):
    return {f["id"]: f for f in json.loads(Path(path).read_text())["fixtures"]}


def fmt_eff(eff):
    return "never" if eff == 21 else str(eff)


def main():
    old, new = load(sys.argv[1]), load(sys.argv[2])
    keys = sys.argv[3:] or ["dragon", "pit-fiend", "lich"]
    ids = sorted((i for i in new if any(k in i for k in keys)), key=lambda i: (new[i]["cr"], i))

    print("## Per fixture\n")
    print("| Fixture | CR | Old effective | New effective | Old gap | New gap | Shift |")
    print("| --- | --- | --- | --- | --- | --- | --- |")
    for i in ids:
        cr = new[i]["cr"]
        o = effective(old[i]) if i in old else None
        n = effective(new[i])
        og = "" if o is None else f"{o - cr:+g}"
        shift = "" if o is None else f"{n - o:+d}"
        print(f"| {i} | {cr:g} | {'' if o is None else fmt_eff(o)} | {fmt_eff(n)} | {og} | {n - cr:+g} | {shift} |")

    print("\n## Gap by fixture CR tier\n")
    print("Gaps that include a fixture never reading Low are lower bounds, marked \"or more\".\n")
    print("| Fixture CR | Group | Fixtures | Old median gap | New median gap | New: at or below CR |")
    print("| --- | --- | --- | --- | --- | --- |")
    groups = [("dragon", lambda i: "dragon" in i), ("non-dragon", lambda i: "dragon" not in i)]
    for lo, hi in TIERS:
        for name, keep in groups:
            members = [i for i in new if lo <= new[i]["cr"] <= hi and keep(i)]
            if not members:
                continue
            cells = []
            for curve in (old, new):
                effs = [(effective(curve[i]), curve[i]["cr"]) for i in members if i in curve]
                gaps = [e - cr for e, cr in effs]
                bound = " or more" if any(e == 21 for e, _ in effs) else ""
                cells.append(f"{statistics.median(gaps):+g}{bound}")
            below = sum(1 for i in members if effective(new[i]) <= new[i]["cr"])
            print(f"| {lo}-{hi} | {name} | {len(members)} | {cells[0]} | {cells[1]} | {below} of {len(members)} |")

    print("\n## CR 19 and up at levels 17 to 20\n")
    print("Wins and deaths out of 100 fights, old curve then new curve.\n")
    print("| Fixture | CR | " + " | ".join(f"L{l}" for l in HIGH_LEVELS) + " |")
    print("| --- | --- | " + " | ".join("---" for _ in HIGH_LEVELS) + " |")
    for i in sorted((i for i in new if new[i]["cr"] >= 19), key=lambda i: (new[i]["cr"], i)):
        row = []
        for level in HIGH_LEVELS:
            pair = []
            for curve in (old, new):
                c = next(c for c in curve[i]["cells"] if c["level"] == level)
                pair.append(f"{c['wins']}w {c['deaths']}d")
            row.append(" to ".join(pair))
        print(f"| {i} | {new[i]['cr']:g} | " + " | ".join(row) + " |")


if __name__ == "__main__":
    main()
