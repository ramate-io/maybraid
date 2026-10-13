#!/usr/bin/env python3
"""Parse `footprint <pid>` into phys_footprint and GPU-ish bytes.

Prints: phys_bytes,gpu_bytes
Unknown fields are left empty. Units accepted: B, K/KB/KiB, M/MB/MiB, G/GB/GiB.
"""

from __future__ import annotations

import re
import sys

UNIT = {
	"b": 1,
	"k": 1024,
	"kb": 1024,
	"kib": 1024,
	"m": 1024**2,
	"mb": 1024**2,
	"mib": 1024**2,
	"g": 1024**3,
	"gb": 1024**3,
	"gib": 1024**3,
}

SIZE = re.compile(
	r"(?P<n>[-+]?\d+(?:\.\d+)?)\s*(?P<u>GiB|MiB|KiB|GB|MB|KB|G|M|K|B)",
	re.IGNORECASE,
)


def parse_size(raw: str) -> float | None:
	match = SIZE.search(raw)
	if not match:
		return None
	return float(match.group("n")) * UNIT[match.group("u").lower()]


def main() -> int:
	text = sys.stdin.read()
	phys = None
	gpu = 0.0
	gpu_seen = False
	for raw in text.splitlines():
		line = raw.strip()
		low = line.lower()
		if low.startswith("phys_footprint") or "phys_footprint" in low:
			phys = parse_size(line)
			continue
		if any(key in low for key in ("gpu dirty", "gpu_dirty", "ioaccelerator", "iosurface")):
			value = parse_size(line)
			if value is not None:
				gpu += value
				gpu_seen = True
	phys_s = f"{phys:.0f}" if phys is not None else ""
	gpu_s = f"{gpu:.0f}" if gpu_seen else ""
	sys.stdout.write(f"{phys_s},{gpu_s}\n")
	return 0


if __name__ == "__main__":
	raise SystemExit(main())
