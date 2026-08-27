# Contributing (`chico-forests`)

Forests assemble **concrete grove types** from `chico-groves`. Do not add a `TuftsGrove` / `UpperCanopyGrove` enum in groves.

- Forest cell: [`DEFAULT_FOREST_EXTENT_XZ`](src/extent.rs) (1600 m).
- Grove tile: [`DEFAULT_FOREST_GROVE_TILE_XZ`](src/extent.rs) (100 m, same as grove preview).
- Hopscotch: [`chico_hopscotch`](src/chico.rs) ([RFC-183 §3.5.5](../../../rfc/rfc-000-000-183-chico-vegetation/03-05-cellular-forests/05-chico-vegetation/README.md)).
- Layer recipes: [`layerings.rs`](src/layerings.rs). Ground cover is omitted. Conifer Lower Massives is missing — drop that bucket; do not alias Alpine or Conifer Massives.
- Grow with `Params::default().with_extent(tile).build_on(world)`. Do not pass CLI grove noise into `build_unit`.
- Planting cells are **world-aligned** ([`GroveExtent::cells_overlapping`](../groves/src/grove/extent.rs)). Adjacent tiles of the same grove share one lattice; a tile owns a cell when the cell center is in its half-open XZ footprint.
- Every presenting 100 m tile softmax-blends a cardinal run of **produced grove** slots ([`blend.rs`](src/blend.rs), radius 8). Each slot's kind is whoever produced that grove. Same-kind slots share one logit (best influence) so a uniform block does not drown the seam. Interior tiles whose slots are all one planted kind skip softmax and grow that recipe once. Empty layers still present neighbor islands. Stream cache is still adjacent forest selections. See [`presenting_recipes`](src/assemble.rs) + [`NeighborLayers`](src/assemble.rs).
- Do not implement the full RFC `ForestGroveBiases` set yet. `select_cell` leaves grove biases at default.
- [`ChicoForest`](src/forest.rs) is a **select-only** generation dependency (`SelectedLayers` on a 1600 m cell). Do not implement `LodScene` on it. Do not bake grove tiles in `build_with_id`.
- [`ChicoGrove`](src/grove.rs) is the generate origin: one id per (100 m tile, layer). `build_with_id` `get_or_generate`s the parent forest (and cardinal neighbors), then stores blend recipes. Growing plants is presentation (or a later off-thread fulfill), not generate.
- A [`ForestIndex`](src/index.rs) stores both forests and groves. Playground presents `ChicoGrove`; the scene stack LOD-refreshes those hosts. Generate, present, and scene plugins stay independently loaded.
- Default stream rings: generate [`GROVE_GENERATE_RADIUS_M`](src/generation.rs) (2 km), present [`GROVE_PRESENT_RADIUS_M`](src/generation.rs) (1 km).
- Pinned review cells use [`ForestLayering::typical_layers`](src/kind.rs) (highest-weight non-`None` grove per layer). Hopscotch cells still Bucket-Throw.

File shape: no `mod.rs`. Methods live on `ForestExtent`, `ChicoForest`, `ChicoGrove`, `LayeringKind`, and the assemble helpers.
