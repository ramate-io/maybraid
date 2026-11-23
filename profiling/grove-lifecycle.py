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


# Area 1746 is the published longwalk-checkpoints-r2 origin. Straight +Z from
# that spawn is not walkable: the native scout at 3dbe964 stuck 9.486 m short of
# (1746.8, 1087.9) with the player airborne (issuecomment-7090987541).
#
# Grounded player positions below are the v0.1.5-traces ledger
# e9542c4_20261117_m4pro-macmini_longwalk-checkpoints-r2.csv (source e9542c4).
# That walk went northeast along a ridge, not +Z. Stop at outbound-08, the last
# clean outbound sample before obstacle sidesteps, airborne detours, and Home.
# The previous native 15-waypoint grove walk held x=-4950, about 7 km west.
AREA_1746 = (1746.8, 1037.9)
START = AREA_1746
GOTO_TIMEOUT_S = 100
GOTO_RADIUS_M = 6
# Ordinary walk, matching the completed x=-4950 grove route. Radius 6 covers
# the ~30 m weave between published samples; radius 3 is what failed on +Z.
LONGWALK_TRACES = (
    "https://github.com/ramate-io/maybraid/releases/download/v0.1.5-traces/"
    "e9542c4_20261117_m4pro-macmini_longwalk-checkpoints-r2.csv"
)
FAILED_PLUS_Z_FIRST = (1746.8, 1087.9)
PRESENT_RADIUS_M = 1000.0

# (x, z, census). Names match the published CSV checkpoint / path_milestone.
APPROACH = (
    (1760.535, 1060.718, False),  # longwalk-probe-forward, 26.6 m path
    (1833.129, 1187.796, True),  # active_grounded_200m
    (1833.985, 1190.646, False),  # longwalk-outbound-forward-01
    (1872.869, 1328.336, True),  # active_grounded_400m
    (1880.571, 1342.119, False),  # longwalk-outbound-forward-02
    (1941.966, 1507.476, True),  # active_grounded_600m / outbound-forward-03
)
CULL = (
    (1955.541, 1572.125, False),  # longwalk-outbound-forward-04
    (1879.133, 1596.722, False),  # longwalk-ridge-lateral-probe
    (1882.193, 1605.693, False),  # active_grounded_800m
    (1910.548, 1728.830, False),  # longwalk-outbound-north-01
    (1924.417, 1749.852, True),  # active_grounded_1000m
    (1984.378, 1844.725, False),  # longwalk-outbound-forward-05
    (1948.433, 1866.221, False),  # longwalk-ridge-side-step-01
    (1973.590, 1902.992, False),  # active_grounded_1200m
    (2002.410, 1940.022, False),  # longwalk-ridge-forward-after-step
    (2081.021, 2054.770, False),  # active_grounded_1400m
    (2113.624, 2107.524, False),  # longwalk-outbound-forward-07
    (2178.791, 2216.279, False),  # longwalk-outbound-forward-08, 1255 m net
)
REENTRY = tuple((x, z, False) for x, z, _ in reversed(CULL[:-1])) + (
    (APPROACH[-1][0], APPROACH[-1][1], False),
)


def xz_of(point):
    return [point[0], point[1]]


def hypot_xz(a, b):
    return math.dist((a[0], a[1]), (b[0], b[1]))


def polyline_m(points):
    return sum(hypot_xz(a, b) for a, b in zip(points, points[1:]))


def goto_points():
    return [START] + [p[:2] for p in APPROACH + CULL + REENTRY]


def planned_distance_m():
    return polyline_m(goto_points())


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

    def walk(name, points):
        mark(name, "start")
        for x, z, snapshot in points:
            rows.append(dict(cmd="goto", xz=[x, z], timeout=GOTO_TIMEOUT_S,
                             radius=GOTO_RADIUS_M, sprint=False))
            if census and snapshot:
                rows.append(dict(cmd="console", line="diagnostics memory"))
                rows.append(dict(cmd="wait", secs=2))
        mark(name, "end")

    wait("loaded", 8)
    walk("approach", APPROACH)
    rows.append(dict(cmd="console", line="diagnostics view 2.094395 0.47"))
    wait("steady-warmup", 3)
    wait("steady", 15)
    walk("cull", CULL)
    wait("culled", 8)
    walk("reentry", REENTRY)
    rows.append(dict(cmd="console", line="diagnostics view 2.094395 0.47"))
    wait("returned-warmup", 3)
    wait("returned", 15)
    rows.append(dict(cmd="quit"))
    return rows


def script_header():
    return (
        "# Agent API 10+: Area 1746 ridge from published longwalk-checkpoints-r2 "
        f"({LONGWALK_TRACES}); not +Z. Native +Z scout stuck 9.486 m short of "
        f"{FAILED_PLUS_Z_FIRST[0]},{FAILED_PLUS_Z_FIRST[1]}. "
        f"radius={GOTO_RADIUS_M} sprint=false. Scout before pairing.\n"
    )


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
                planned_distance_m=round(planned_distance_m(), 1),
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
            stream.write(script_header())
            for row in route(args.census):
                stream.write(json.dumps(row, separators=(",", ":")) + "\n")
    else:
        print(json.dumps(inspect(args.events), indent=2))


if __name__ == "__main__":
    main()
