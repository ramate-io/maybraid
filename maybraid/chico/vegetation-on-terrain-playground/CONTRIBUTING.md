# Contributing to vegetation-on-terrain

Library host for Durham vegetation: character/camera, grove/forest stream drivers,
and (when `own_terrain`) a small fine-grid patch. Assembled world runs
[`maybraid-world-playground`](../../world/playground/). This crate is no longer a
standalone binary; restore the retired app from [PLAYGROUNDS.md](../../PLAYGROUNDS.md).

`/forest` streams Chico groves grown on Durham height. `/grove` keeps the tiled
one-kind path for A/B. High-band stick / trunk capsules are on for character
physics. Water exclusion stays out.

## Layout

- Fine cells only (no macro rings) when the crate owns terrain.
- Default `r = 2` → 4×4 cells (~640 m at `TERRAIN_CELL_SIZE`).
- One grove type at a time, tiled by `grove-extent` × `tile-radius`.
- `/forest` expands `terrain-radius` to cover the 3 km grove generate ring when needed. World coverage keeps the [#675](https://github.com/ramate-io/maybraid/pull/675) fine disk (16 cells, ~2.6 km); canopy bump-outs clone those cell mesh handles and do not expand generate.

## Verify

```bash
cargo check -p chico-vegetation-on-terrain-playground
```
