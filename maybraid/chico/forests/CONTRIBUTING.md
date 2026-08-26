# Contributing (`chico-forests`)

Forests assemble **concrete grove types** from `chico-groves`. Do not add a `TuftsGrove` / `UpperCanopyGrove` enum in groves.

- Forest cell: [`DEFAULT_FOREST_EXTENT_XZ`](src/extent.rs) (1600 m).
- Grove tile: [`DEFAULT_FOREST_GROVE_TILE_XZ`](src/extent.rs) (100 m, same as grove preview).
- Hopscotch: [`chico_hopscotch`](src/chico.rs) ([RFC-183 §3.5.5](../../../rfc/rfc-000-000-183-chico-vegetation/03-05-cellular-forests/05-chico-vegetation/README.md)).
- Layer recipes: [`layerings.rs`](src/layerings.rs). Ground cover is omitted. Conifer Lower Massives is missing — drop that bucket; do not alias Alpine or Conifer Massives.
- Grow with `Params::default().with_extent(tile).build_on(world)`. Do not pass CLI grove noise into `build_unit`.
- Do not implement the full RFC `ForestGroveBiases` set yet. `select_cell` leaves grove biases at default.
- [`ChicoForest`](src/forest.rs) is a generation result. Do not implement `LodScene` on it. Playgrounds spawn the concrete grove hosts underneath.

File shape: no `mod.rs`. Methods live on `ForestExtent`, `ChicoForest`, `LayeringKind`, and the assemble helpers.
