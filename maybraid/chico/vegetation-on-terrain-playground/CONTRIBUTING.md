# Contributing to the vegetation-on-terrain playground

Small Durham fine-grid patch for iterating Chico groves on real ground.
The binary still opens in free-look; `/mode character` and `/set-character`
reuse the Durham capsule + Crozon hosts. High-band stick / trunk capsules
(≥ 4 in) are on for character physics. Water exclusion stays out.

`/forest` streams the same generate / present / cull plugins as SBS `/show
forest`, but grows tiles on Durham height so they sit on the mesh. `/grove`
keeps the tiled one-kind path for A/B.

## Layout

- Fine cells only (no macro rings). Chebyshev half-extent is live (`terrain-radius`).
- Default `r = 2` → 4×4 cells (~640 m at `TERRAIN_CELL_SIZE`).
- One grove type at a time, tiled by `grove-extent` × `tile-radius`.
- `/forest` expands `terrain-radius` to cover the 3 km generate ring when needed.

## Run

```bash
cargo run -p chico-vegetation-on-terrain-playground
cargo run -p chico-vegetation-on-terrain-playground -- grove rolling-oaks
cargo run -p chico-vegetation-on-terrain-playground -- forest
cargo run -p chico-vegetation-on-terrain-playground -- forest lush-jungle
```

In-game: `/` console, `Y` or `F1` drawer. Fly camera: WASD, Space/Shift, mouse look.
Character: `/mode character`, WASD, mouse look, Space jump.

- `grove <kind>` — tiled groves (disables forest stream)
- `forest [layering] [--stream-radius N] [--noise …]` — stream forest (disables tiled groves)
- `terrain-radius <cells>`
- `grove-extent <meters>`
- `tile-radius <tiles>`
- `rebuild`
- `mode free|character`
- `set-character <species>`
- `stats mesh` — triangle / probe / LOD-host counts (also logged)

Defaults: `monster-grass`, `terrain-radius 2` (4×4 cells), `grove-extent 100`, `tile-radius 1` (3×3 tiles).

Throttled FPS logging is off by default. Enable with `CHICO_VEG_TERRAIN_DIAG=fps`
(`[veg.timing]` once per second).

## Verify

```bash
cargo check -p chico-vegetation-on-terrain-playground
```
