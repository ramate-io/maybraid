# Tree-canopy playground

Deterministic viewer for GPU-driven BotW-like tree canopies stacked on
[#1641](https://github.com/ramate-io/maybraid/pull/1641) /
[#1643](https://github.com/ramate-io/maybraid/pull/1643).

```bash
nix develop --command cargo run -p vegetation-tree-field-playground --release --locked
```

WASD + mouse look (`L` toggles look). `/` opens commands. `F1` toggles the drawer.

## Commands

| Command | What it does |
| --- | --- |
| `spawn default\|close\|grove\|slope\|leafless\|sparse\|lod\|clear` | Patch ownership (not per-branch entities) |
| `camera orbit\|overhead\|underside\|ground` | Reproducible routes |
| `wind still\|gentle\|set --strength --speed` | Anchored travelling waves, stiffer wood |
| `lod host high\|medium\|low\|ultra-low` | Existing LOD host density scale |
| `shadows proxy\|full\|off` | Production default is proxy |
| `compare turf\|gpu\|kit on\|off` | Terrain mat vs GPU vs Storybook kit preview |

## Captures

960×540 stills in [`captures/`](captures/). `kit-*` shots use the same camera as
the matching GPU shot, with GPU patches replaced by the existing Storybook Tree
ball-stick path. Grass from [#1641](https://github.com/ramate-io/maybraid/pull/1641)
is used as ground cover.

| File | View |
| --- | --- |
| `canopy-orbit.png` | Orbit around a close storybook tree |
| `kit-orbit.png` | Same camera, authored Storybook Tree kit |
| `canopy-overhead.png` | Crown silhouette from above |
| `canopy-underside.png` | Ground-level looking up into the canopy |
| `wood-leafless.png` | Branch skeleton without foliage |
| `canopy-close.png` | Layered canopy close-up |
| `kit-close.png` | Same camera, authored Storybook Tree kit |
| `grove-edge.png` | Small grove / forest edge |
| `slope-rake.png` | Trees following a slope normal |
| `lod-transition.png` | Near / mid / far in one view |
| `wind-gust.png` | Frozen travelling gust |

```bash
MAYBRAID_CAPTURE_DIR=maybraid/world/layers/vegetation/tree-field-playground/captures \
  nix develop --command cargo run -p vegetation-tree-field-playground --release --locked
```

Refresh these PNGs whenever the look changes.

## Honest measurements

Taken from the CPU-mirrored select path (same math as `select_wood.wesl` /
`select_foliage.wesl`) on a 36 m grove at 12 m spacing, 720p, 55° FOV.
**Not** a GPU timer capture — this environment has no M1-class Apple Silicon
sample.

Resident memory is `plants × 96 B + branches × 80 B + leaves × 64 B` plus six
LOD lists (`max_per_lod × 4 B`) and 120 B of indirect args per view.

CPU-mirrored 36 m grove at 12 m spacing, 720p, looking from ~22 m:

| | Count |
| --- | --- |
| Resident plants / branches / leaves | 9 / 216 / 4414 |
| Submitted wood / foliage instances | 199 / 3986 |
| Submitted triangles | 45 414 |
| Descriptor bytes | 300 640 |

Do not read triangle reduction as a raster-cost win by itself: far foliage
uses fewer leaves but larger size scale. Apple Silicon / M1-class GPU timers:
**not measured here**. Leaf count approaches `MAX_LEAVES_PER_PLANT` (512) per
tree; outer canopy shells are packed across every limb first so a clamp keeps
the crown.
