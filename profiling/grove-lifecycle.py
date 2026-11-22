#!/usr/bin/env python3
"""Prepare or inspect the natural grove lifecycle route; never launches a game.

Use the same generated script for every arm. The proposed route must be scouted
with ordinary physics before admitting paired timings. Failed routes remain
useful evidence but are not a completed multi-kilometre benchmark.
"""

import argparse
import json
import math
from pathlib import Path


# Area 1746 / published longwalk-checkpoints-r2 start. The previous native
# route held x=-4950, about 7 km west of this neighborhood.
AREA_1746 = (1746.8, 1037.9)
START = AREA_1746
X = START[0]
Z0 = START[1]
BANDS_M = (50, 200, 450, 900)


def z_at(delta_m):
    return Z0 + delta_m


def route(census=False):
    rows = [dict(cmd="start", xz=list(START)),
            dict(cmd="console", line="diagnostics on"),
            dict(cmd="wait_for", flow="world", pause="playing", timeout=90),
            dict(cmd="console", line="sky noon")]

    def mark(name, side):
        rows.append(dict(cmd="scenario", name="grove-" + name, mark=side))

    def wait(name, seconds):
        mark(name, "start")
        rows.append(dict(cmd="wait", secs=seconds))
        mark(name, "end")
        if census and name in ("loaded", "steady", "culled", "returned"):
            rows.append(dict(cmd="console", line="diagnostics memory"))
            rows.append(dict(cmd="wait", secs=2))

    def maybe_band_census(delta_m):
        if census and delta_m in BANDS_M:
            rows.append(dict(cmd="console",
                             line="diagnostics memory"))
            rows.append(dict(cmd="wait", secs=2))

    def walk(name, deltas):
        mark(name, "start")
        for delta_m in deltas:
            rows.append(dict(cmd="goto", xz=[X, z_at(delta_m)], timeout=100,
                             radius=3, sprint=False))
            maybe_band_census(delta_m)
        mark(name, "end")

    wait("loaded", 8)
    walk("approach", [50, 200, 450, 600])
    rows.append(dict(cmd="console", line="diagnostics view 2.094395 0.47"))
    wait("steady-warmup", 3)
    wait("steady", 15)
    walk("cull", [900, 1100, 1300, 1500, 1700, 1800])
    wait("culled", 8)
    walk("reentry", [1600, 1400, 1200, 1000, 800, 600])
    rows.append(dict(cmd="console", line="diagnostics view 2.094395 0.47"))
    wait("returned-warmup", 3)
    wait("returned", 15)
    rows.append(dict(cmd="quit"))
    return rows


def inspect(path):
    # A live writer may leave a partial final line; preserve that fact.
    raw = path.read_text()
    lines = raw.splitlines()
    complete_lines = lines if raw.endswith("\n") else lines[:-1]
    events = [json.loads(line) for line in complete_lines if line.strip()]
    failures = [r for r in events if r.get("status") == "failed"
                or r.get("kind") in ("died", "invariant")]
    players = [r for r in events if r.get("kind") == "player"]
    phases = {}
    for event in events:
        if event.get("kind") != "scenario" or not event.get("name", "").startswith("grove-"):
            continue
        phase = phases.setdefault(event["name"], {})
        phase[event["mark"]] = event["t"]
    for name, phase in phases.items():
        lo, hi = phase.get("start"), phase.get("end")
        if lo is None:
            continue
        sample = [r for r in players if r["t"] >= lo and (hi is None or r["t"] <= hi)]
        phase["complete"] = hi is not None
        phase["duration_s"] = hi - lo if hi is not None else None
        phase["player_samples"] = len(sample)
        phase["observed_path_m"] = sum(math.dist((a["x"], a["z"]), (b["x"], b["z"]))
                                       for a, b in zip(sample, sample[1:]))
        phase["pending_support_samples"] = sum(
            bool((r.get("vitals") or {}).get("pending_support")) for r in sample)
        phase["airborne_samples"] = sum(
            (r.get("vitals") or {}).get("grounded") is False for r in sample)
        phase["minimum_health"] = min(((r.get("vitals") or {}).get("health", math.inf)
                                       for r in sample), default=None)
        if sample:
            phase["first_xz"] = [sample[0][k] for k in ("x", "z")]
            phase["last_xz"] = [sample[-1][k] for k in ("x", "z")]
    expected = {r["name"] for r in route() if r["cmd"] == "scenario"}
    missing = sorted(name for name in expected if not phases.get(name, {}).get("complete"))
    quit_seen = any(r.get("kind") == "command" and r.get("cmd") == "quit"
                    and r.get("status") == "finished" for r in events)
    return dict(phases=phases, failures=failures, incomplete_phases=missing,
                trailing_partial_line=not raw.endswith("\n"), quit_seen=quit_seen,
                complete=not failures and not missing,
                planned_distance_m=3000,
                limitation="Sampled path length is not a content-equivalence or culling proof. "
                "Inspect tile identities, collision/support, foreground, actual present mode, "
                "and descriptor admission separately. Loading starts before script playback; "
                "measure process launch to supported readiness from the operator receipts.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    prepare = sub.add_parser("prepare")
    prepare.add_argument("output", type=Path)
    prepare.add_argument("--census", action="store_true",
                         help="separate memory-census build: explicit phase-boundary snapshots")
    check = sub.add_parser("inspect")
    check.add_argument("events", type=Path)
    args = parser.parse_args()
    if args.command == "prepare":
        with args.output.open("x") as stream:
            stream.write(
                "# Agent API 10+: Area 1746 (1746.8, 1037.9) 3 km walk along +Z; "
                "overlay longwalk-checkpoints-r2 if the native checker has that script.\n"
            )
            for row in route(args.census):
                stream.write(json.dumps(row, separators=(",", ":")) + "\n")
    else:
        print(json.dumps(inspect(args.events), indent=2))


if __name__ == "__main__":
    main()
