#!/usr/bin/env python3
"""Summarize a memory-flight CSV (+ optional agent events).

    python3 profiling/summarize-memory.py \
        --samples capture-memory.csv \
        --events capture-events.jsonl \
        --scenario biome-traverse \
        --json out.json --out out.txt

CSV columns: t_s,rss_kb,phys_bytes,gpu_bytes
`phys_bytes` / `gpu_bytes` may be empty when a cheap sample skipped `footprint`.
The measure window is the last non-warmup `scenario` start/end in the events
file (game seconds). Samples use wall seconds from process start; the
summarizer aligns them by treating the first sample as t=0 and using event
`t` as the game clock. When events are missing, the whole series is used.
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
from pathlib import Path


def parse_args() -> argparse.Namespace:
	parser = argparse.ArgumentParser()
	parser.add_argument("--samples", required=True, type=Path)
	parser.add_argument("--events", type=Path)
	parser.add_argument("--scenario")
	parser.add_argument("--json", type=Path)
	parser.add_argument("--out", type=Path)
	return parser.parse_args()


def load_samples(path: Path) -> list[dict]:
	rows: list[dict] = []
	for i, raw in enumerate(path.read_text(errors="replace").splitlines()):
		line = raw.strip()
		if not line or line.startswith("#"):
			continue
		if i == 0 and line.startswith("t_s"):
			continue
		parts = [p.strip() for p in line.split(",")]
		if len(parts) < 2:
			continue
		try:
			t_s = float(parts[0])
			rss_kb = float(parts[1]) if parts[1] else None
		except ValueError:
			continue
		phys = _opt_float(parts[2] if len(parts) > 2 else "")
		gpu = _opt_float(parts[3] if len(parts) > 3 else "")
		rows.append({"t_s": t_s, "rss_kb": rss_kb, "phys_bytes": phys, "gpu_bytes": gpu})
	return rows


def _opt_float(raw: str) -> float | None:
	raw = raw.strip()
	if not raw:
		return None
	try:
		return float(raw)
	except ValueError:
		return None


def scenario_window_s(events: Path | None, scenario: str | None) -> tuple[float | None, float | None]:
	if events is None or scenario is None or not events.is_file():
		return None, None
	start = end = None
	for raw in events.read_text(errors="replace").splitlines():
		line = raw.strip()
		if not line:
			continue
		try:
			ev = json.loads(line)
		except json.JSONDecodeError:
			continue
		if ev.get("kind") != "scenario":
			continue
		if ev.get("name") != scenario:
			continue
		t = ev.get("t")
		if t is None:
			continue
		if ev.get("mark") == "start":
			start = float(t)
		elif ev.get("mark") == "end":
			end = float(t)
	return start, end


def series(rows: list[dict], key: str) -> list[float]:
	return [row[key] for row in rows if row.get(key) is not None]


def stats(values: list[float]) -> dict:
	if not values:
		return {"n": 0}
	steady = values[max(0, int(len(values) * 0.8)) :]
	return {
		"n": len(values),
		"min": min(values),
		"max": max(values),
		"mean": statistics.fmean(values),
		"median": statistics.median(values),
		"last": values[-1],
		"first": values[0],
		"growth": values[-1] - values[0],
		"steady_median": statistics.median(steady) if steady else values[-1],
		"steady_mean": statistics.fmean(steady) if steady else values[-1],
	}


def fmt_bytes(n: float | None) -> str:
	if n is None:
		return "—"
	sign = "-" if n < 0 else ""
	n = abs(n)
	for unit, step in (("GiB", 1024**3), ("MiB", 1024**2), ("KiB", 1024)):
		if n >= step:
			return f"{sign}{n / step:.1f} {unit}"
	return f"{sign}{n:.0f} B"


def fmt_kb(n: float | None) -> str:
	if n is None:
		return "—"
	return fmt_bytes(n * 1024.0)


def main() -> int:
	args = parse_args()
	rows = load_samples(args.samples)
	if not rows:
		print(f"no samples in {args.samples}", file=sys.stderr)
		return 1

	win_start, win_end = scenario_window_s(args.events, args.scenario)
	window_rows = rows
	if win_start is not None:
		# Agent `t` is game seconds from boot; samples are wall seconds from
		# process start. They are close enough for a flight (no vsync toggle).
		end = win_end if win_end is not None else rows[-1]["t_s"]
		window_rows = [row for row in rows if win_start <= row["t_s"] <= end]
		if not window_rows:
			window_rows = rows

	rss = stats(series(window_rows, "rss_kb"))
	phys = stats(series(window_rows, "phys_bytes"))
	gpu = stats(series(window_rows, "gpu_bytes"))
	all_rss = stats(series(rows, "rss_kb"))
	all_phys = stats(series(rows, "phys_bytes"))

	payload = {
		"scenario": args.scenario,
		"samples": len(rows),
		"window_samples": len(window_rows),
		"window_s": [win_start, win_end],
		"span_s": [rows[0]["t_s"], rows[-1]["t_s"]],
		"window": {
			"rss_kb": rss,
			"phys_bytes": phys,
			"gpu_bytes": gpu,
		},
		"whole": {
			"rss_kb": all_rss,
			"phys_bytes": all_phys,
		},
	}

	lines = [
		f"scenario={args.scenario or 'whole-flight'}",
		f"samples={len(rows)} window_samples={len(window_rows)} "
		f"span={rows[0]['t_s']:.1f}–{rows[-1]['t_s']:.1f}s",
		"",
		"measure window",
		f"  RSS peak={fmt_kb(rss.get('max'))}  last={fmt_kb(rss.get('last'))}  "
		f"steady_median={fmt_kb(rss.get('steady_median'))}  growth={fmt_kb(rss.get('growth'))}",
		f"  phys peak={fmt_bytes(phys.get('max'))}  last={fmt_bytes(phys.get('last'))}  "
		f"steady_median={fmt_bytes(phys.get('steady_median'))}  growth={fmt_bytes(phys.get('growth'))}",
		f"  GPU  peak={fmt_bytes(gpu.get('max'))}  last={fmt_bytes(gpu.get('last'))}  "
		f"growth={fmt_bytes(gpu.get('growth'))}",
		"",
		"whole flight",
		f"  RSS peak={fmt_kb(all_rss.get('max'))}  last={fmt_kb(all_rss.get('last'))}  "
		f"growth={fmt_kb(all_rss.get('growth'))}",
		f"  phys peak={fmt_bytes(all_phys.get('max'))}  last={fmt_bytes(all_phys.get('last'))}  "
		f"growth={fmt_bytes(all_phys.get('growth'))}",
	]
	text = "\n".join(lines) + "\n"

	if args.out:
		args.out.write_text(text)
	else:
		sys.stdout.write(text)
	if args.json:
		args.json.write_text(json.dumps(payload, indent=2) + "\n")
	return 0


if __name__ == "__main__":
	raise SystemExit(main())
