# `renderit`

Refined **render dispatch** API (successor to the hard-coded `CascadeChunk` + `Transform` path in `util/render-item`).

- **`DispatchRenderItem<T>`** — insert on an entity to start a handling chain; responses spawn as **children** of that entity.
- **`RenderItem<Ctx>`** — your payload; `Ctx` comes from the same query row (any `QueryData` shape).
- **`RenderDispatchSource`** — extract `Entity`, clone `DispatchRenderItem`, and clone `Ctx` from each row; use with [`process_render_dispatches`] and [`RenderDispatchPlugin`].
- **`process_render_dispatches_simple`** — shortcut for `(Entity, &DispatchRenderItem<I>, &Ctx)` + `Added<DispatchRenderItem<I>>`.

SDF mesh pathway: [`SdfRenderContext`], [`SdfMeshPayload`], and [`spawn_sdf_placeholder_cuboid_child`] (real `Assets<Mesh>` cuboid). Placeholder `RenderItem` spawn uses a named child entity until voxelization lands.

Disk/cache wrappers: [`wrappers`] module (stubs; see `util/render-item` `mesh::cache` during migration).
