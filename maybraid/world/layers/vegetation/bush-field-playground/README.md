# Small-bush playground

Deterministic viewer for GPU-driven BotW-like woody bushes
([#1638](https://github.com/ramate-io/maybraid/issues/1638)).

```bash
nix develop --command cargo run -p vegetation-bush-field-playground --release --locked
```

WASD + mouse look (`L` toggles look). `/` opens commands. `F1` toggles the drawer.

## Commands

| Command | What it does |
| --- | --- |
| `spawn default\|close\|dense\|slope\|leafless\|sparse\|clear` | Patch ownership (not per-branch entities) |
| `camera orbit\|overhead\|underside\|ground` | Reproducible routes |
| `wind still\|gentle\|set --strength --speed` | Anchored travelling waves, stiffer wood |
| `lod host high\|medium\|low\|ultra-low` | Existing LOD host density scale |
| `shadows proxy\|full\|off` | Production default is proxy |
| `compare turf\|gpu\|kit on\|off` | Terrain mat vs GPU vs ball-stick kit preview |

## Captures

960×540 stills in [`captures/`](captures/). `kit-*` shots use the same camera as
the matching GPU shot, with GPU patches replaced by the existing ball-stick
small-bush topology (tapered sticks + crown balls).

| File | View |
| --- | --- |
| `bush-orbit.png` | Orbit around a close scrub bush |
| `kit-orbit.png` | Same camera, authored ball-stick kit |
| `bush-overhead.png` | Crown silhouette from above |
| `bush-underside.png` | Attached wood + two-sided leaves from below |
| `wood-leafless.png` | Sparse woody stems without foliage |
| `bush-close.png` | Full bush close-up |
| `kit-close.png` | Same camera, authored ball-stick kit |
| `dense-patch.png` | Dense 1.35 m scrub patch |
| `slope-rake.png` | Bushes following a slope normal |
| `lod-transition.png` | Near / mid / far in one view |
| `wind-gust.png` | Frozen travelling gust |

```bash
MAYBRAID_CAPTURE_DIR=maybraid/world/layers/vegetation/bush-field-playground/captures \
  nix develop --command cargo run -p vegetation-bush-field-playground --release --locked
```

Refresh these PNGs whenever the look changes.

## Honest measurements

Taken from the CPU-mirrored select path (same math as `select_wood.wesl` /
`select_foliage.wesl`) on a 10 m scrub patch at 1.35 m spacing, 720p, 55° FOV.
**Not** a GPU timer capture — this environment has no M1-class Apple Silicon
sample.

Resident memory is `plants × 96 B + branches × 80 B + leaves × 64 B` plus six
LOD lists (`max_per_lod × 4 B`) and 120 B of indirect args per view.

CPU-mirrored 10 m scrub at 1.35 m spacing, 720p, looking from ~8 m:

| | Count |
| --- | --- |
| Resident plants / branches / leaves | 64 / 578 / 2888 |
| Submitted wood / foliage instances | 308 / 1558 |
| Submitted triangles | 19 748 |
| Descriptor bytes | 237 216 |

Do not read triangle reduction as a raster-cost win by itself: far foliage
uses fewer leaves but larger size scale. Apple Silicon / M1-class GPU timers:
**not measured here**.
