# Maybraid rendering

## `renderit`

Generic **render dispatch** (see [`renderit`](renderit/) crate): `DispatchRenderItem` + `RenderItem<Context>` + `RenderDispatchSource` (Bevy `QueryData` pattern, comparable to cascade production’s source trait). LOD and chunk normalization stay in game crates, not here.

Legacy mesh/cache helpers remain in [`util/render-item`](../../util/render-item) until callers migrate.
