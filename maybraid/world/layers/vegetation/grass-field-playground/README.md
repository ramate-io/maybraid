# Grass field playground

Deterministic viewer for GPU-driven BotW-like meadow grass
([#1637](https://github.com/ramate-io/maybraid/issues/1637)).

```bash
nix develop --command cargo run -p vegetation-grass-field-playground --release --locked
```

WASD + mouse look (`L` toggles look). `/` opens commands. `F1` toggles the drawer.

## Commands

| Command | What it does |
| --- | --- |
| `spawn default\|meadow\|clump\|slope\|crest\|clear` | Patch ownership (not per-blade entities) |
| `camera orbit\|overhead\|ground` | Reproducible routes |
| `wind still\|gentle\|set --strength --speed` | Anchored travelling waves |
| `lod host high\|medium\|low\|ultra-low` | Existing LOD host density scale |
| `shadows proxy\|full\|off` | Production default is proxy (no grass casters) |
| `compare turf on\|off` / `compare grass on\|off` | Terrain mat vs generated ribbons |

## Captures

960×540 stills in [`captures/`](captures/). `turf-*` shots use the same camera as
the matching grass shot, with GPU blades removed so only the terrain POM turf
mat remains.

```bash
MAYBRAID_CAPTURE_DIR=maybraid/world/layers/vegetation/grass-field-playground/captures \
  nix develop --command cargo run -p vegetation-grass-field-playground --release --locked
```

| File | View |
| --- | --- |
| `meadow-ground.png` | Eye-height, looking across the field |
| `turf-meadow-ground.png` | Same camera, terrain-mat only |
| `blade-close.png` | Low-angle close-up of tapered blades |
| `meadow-overhead.png` | Coverage from above |
| `turf-meadow-overhead.png` | Same camera, terrain-mat only |
| `slope-rake.png` | SDF/slope samples |
| `turf-slope-rake.png` | Same camera, terrain-mat only |
| `lod-transition.png` | Near / mid / far in one view |
| `turf-lod-transition.png` | Same camera, terrain-mat only |
| `wind-gust.png` | Frozen travelling gust, ground camera |
| `meadow-orbit.png` | Orbiting lush field |
| `turf-meadow-orbit.png` | Same camera, terrain-mat only |
| `crest-skyline.png` | Hill crest; height kept at the skyline |
| `clump-close.png` | Isolated clump vs dense field |

Refresh these PNGs whenever the look changes.

## Honest measurements

Taken from the CPU-mirrored select path (same math as `select.wesl`) on a
16 m meadow at 0.10 m spacing, 720p, 55° FOV. **Not** a GPU timer capture —
this environment has no M1-class Apple Silicon sample.

| View | Submitted instances | Triangles | Notes |
| --- | --- | --- | --- |
| Ground ~6 m | dominated by near ribbons | highest | full curvature |
| Orbit ~16 m | mix of near/mid | lower | nested density |
| Overhead ~11 m | mid + cards | lowest of the three | projected height still kept |
| Far ~70 m | cards only, then cull | least | skyline cards remain until `min_height_px` |

Resident memory is `blades × 80 B` plus three LOD lists (`max_per_lod × 4 B`)
and 60 B of indirect args per view. CPU upload is compact descriptors on patch
change, not expanded blade meshes.

Do not read triangle reduction as a raster-cost win by itself: cards spend
fewer vertices but more screen area. Crossed masked cards and analytic
proxies were considered as comparison candidates; production draws opaque
ribbons / cards through the same GPU lists.

Apple Silicon / M1-class numbers: **not measured here**.
