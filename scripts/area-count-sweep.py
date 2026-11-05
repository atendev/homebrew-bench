#!/usr/bin/env python3
"""Rerun fixtures with lower policy area counts and split party damage by action.

Prints Markdown tables used by docs/research/064-dragons-on-the-v3-curve.md.
For each fixture and each area-count setting it runs `bench batch` over levels
1 to 20 and reports the effective CR. At the one level given per fixture it
reports wins, deaths, and the share of damage dealt to the party that came from
actions whose id contains "breath", read from every fight log via `--fight`.

Settings: default (single 1, small 2, medium 3, large 4), large 3, large 2,
medium and large 2, and all 1 (every area 1). Abilities listed in the
committed curve's `ignored_abilities` are ignored, as baseline generation does.
The batch seeds its cells differently from baseline generation, so cells
differ from the curve by sampling noise.

Usage: scripts/area-count-sweep.py [path/to/bench] FIXTURE:LEVEL ...
"""

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CURVE = json.loads((ROOT / "data/baseline/curve.json").read_text())
IGNORED = {f["id"]: f.get("ignored_abilities") or [] for f in CURVE["fixtures"]}
SEED = str(CURVE["base_seed"])
SETTINGS = {
    "default": [],
    "large 3": ["policy.area_counts.large=3"],
    "large 2": ["policy.area_counts.large=2"],
    "medium and large 2": ["policy.area_counts.large=2", "policy.area_counts.medium=2"],
    "all 1": ["policy.area_counts.large=1", "policy.area_counts.medium=1", "policy.area_counts.small=1"],
}
DECISION = re.compile(r"^  DECISION (\S+) (?:action|bonus action|legendary action|reaction)\S*: (\S+)")
DAMAGE = re.compile(r"^\s+(standard-\S+) takes (\d+) damage")


def run(bench, fixture, assumes, *rest):
    args = [bench, "batch", fixture, "--seed", SEED, "--out", f"/tmp/area-count-sweep/{fixture}"]
    for ability in IGNORED[fixture]:
        args += ["--assume", f"abilities.{ability}=ignored"]
    for assume in assumes:
        args += ["--assume", assume]
    out = json.loads(subprocess.run(args + list(rest), capture_output=True, text=True, check=True).stdout)
    if out["status"] != "ok":
        raise SystemExit(json.dumps(out))
    return out["data"]


def breath_share(bench, fixture, level, assumes):
    breath = total = 0
    for fight in range((level - 1) * 100, level * 100):
        actor = action = None
        for line in run(bench, fixture, assumes, "--fight", str(fight))["log"].splitlines():
            if m := DECISION.match(line):
                actor, action = m.groups()
            elif m := DAMAGE.match(line):
                total += int(m.group(2))
                if actor == fixture and "breath" in action:
                    breath += int(m.group(2))
    return breath / total if total else 0.0


def main():
    args = sys.argv[1:]
    bench = args.pop(0) if args and ":" not in args[0] else str(ROOT / "target/release/bench")
    cells = [(a.split(":")[0], int(a.split(":")[1])) for a in args]

    print("## Effective CR by area-count setting\n")
    print("| Fixture | CR | " + " | ".join(SETTINGS) + " |")
    print("| --- | --- | " + " | ".join("---" for _ in SETTINGS) + " |")
    at_level = {}
    for fixture, level in cells:
        effs = []
        for name, assumes in SETTINGS.items():
            data = run(bench, fixture, assumes, "--levels", "1-20")
            eff = next((l["level"] for l in data["levels"] if l["wins"] >= 95 and l["deaths"] < 2), None)
            effs.append("never" if eff is None else str(eff))
            row = next(l for l in data["levels"] if l["level"] == level)
            at_level[(fixture, name)] = (row["wins"], row["deaths"])
        print(f"| {fixture} | {data['claimed_cr']:g} | " + " | ".join(effs) + " |")

    print("\n## One level per fixture\n")
    print("Wins and deaths out of 100 fights, and the breath share of damage dealt to the party.\n")
    print("| Fixture | Level | " + " | ".join(SETTINGS) + " |")
    print("| --- | --- | " + " | ".join("---" for _ in SETTINGS) + " |")
    for fixture, level in cells:
        row = []
        for name, assumes in SETTINGS.items():
            wins, deaths = at_level[(fixture, name)]
            row.append(f"{wins}w {deaths}d, breath {breath_share(bench, fixture, level, assumes):.0%}")
        print(f"| {fixture} | {level} | " + " | ".join(row) + " |")


if __name__ == "__main__":
    main()
