# Contributing (`chico-forests`)

Forests assemble **concrete grove types** from `chico-groves`. Do not add a `TuftsGrove` / `UpperCanopyGrove` enum in groves.

- Forest cell: [`DEFAULT_FOREST_EXTENT_XZ`](src/extent.rs) (1600 m).
- Grove tile: [`DEFAULT_FOREST_GROVE_TILE_XZ`](src/extent.rs) (100 m, same as grove preview).
- Hopscotch: [`chico_hopscotch`](src/chico.rs) ([RFC-183 §3.5.5](../../../rfc/rfc-000-000-183-chico-vegetation/03-05-cellular-forests/05-chico-vegetation/README.md)).
- Layer recipes: [`layerings.rs`](src/layerings.rs). Ground cover is omitted. Conifer Lower Massives is missing — drop that bucket; do not alias Alpine or Conifer Massives.
- Grow with `Params::default().with_extent(tile).build_on(world)`. Do not pass CLI grove noise into `build_unit`.
- Planting cells are **world-aligned** ([`GroveExtent::cells_overlapping`](../groves/src/grove/extent.rs)). Adjacent tiles of the same grove share one lattice; a tile owns a cell when the cell center is in its half-open XZ footprint.
- Every presenting 100 m tile softmax-blends a cardinal run of **produced grove** slots ([`blend.rs`](src/blend.rs), radius 8). Each slot's kind is whoever produced that grove. Same-kind slots share one logit (best influence) so a uniform block does not drown the seam. Empty layers still present neighbor islands. Stream cache is still adjacent forest selections (`R+1` halo, grow `R`). See [`assemble`](src/assemble.rs) + [`NeighborLayers`](src/assemble.rs).
- Do not implement the full RFC `ForestGroveBiases` set yet. `select_cell` leaves grove biases at default.
- [`ChicoForest`](src/forest.rs) is a generation result. Do not implement `LodScene` on it. Playgrounds spawn the concrete grove hosts underneath.
- Pinned review cells use [`ForestLayering::typical_layers`](src/kind.rs) (highest-weight non-`None` grove per layer). Hopscotch cells still Bucket-Throw.

File shape: no `mod.rs`. Methods live on `ForestExtent`, `ChicoForest`, `LayeringKind`, and the assemble helpers.
