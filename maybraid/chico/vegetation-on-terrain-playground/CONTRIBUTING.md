# Contributing to the vegetation-on-terrain playground

Small Durham fine-grid patch for iterating Chico groves on real ground.
Fly camera only. Character / water exclusion / forest-layer constraints stay
in other samples.

## Layout

- Fine cells only (no macro rings). Chebyshev half-extent is live (`terrain-radius`).
- Default `r = 2` → 4×4 cells (~640 m at `TERRAIN_CELL_SIZE`).
- One grove type at a time, tiled by `grove-extent` × `tile-radius`.

## Run

```bash
cargo run -p chico-vegetation-on-terrain-playground
cargo run -p chico-vegetation-on-terrain-playground -- grove rolling-oaks
```

In-game: `/` console, `Y` or `F1` drawer. Fly camera: WASD, Space/Shift, mouse look.

- `grove <kind>`
- `terrain-radius <cells>`
- `grove-extent <meters>`
- `tile-radius <tiles>`
- `rebuild`
- `stats mesh` — triangle / probe / LOD-host counts (also logged)

Defaults: `monster-grass`, `terrain-radius 2` (4×4 cells), `grove-extent 100`, `tile-radius 1` (3×3 tiles).

Throttled FPS logging is off by default. Enable with `CHICO_VEG_TERRAIN_DIAG=fps`
(`[veg.timing]` once per second).

## Verify

```bash
cargo check -p chico-vegetation-on-terrain-playground
```
