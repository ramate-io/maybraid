# Incremental LOD scene chunks

Amortize spawning a **semantic** level root across frames under a weight budget.

[`SemanticLodScene`](../lib/src/scene/lod_scene.rs) (`LodScene` is an alias) builds
[`SceneChunk`](../lib/src/scene/chunk.rs) = `LodChunk<Box<dyn Scene>>`. Drain
spawns those scenes in the main World.

[`VisualLodScene`](../lib/src/scene/visual.rs) is **not** a chunk tree.
[`SceneChunk`](../lib/src/scene/chunk.rs) schedules semantic / world realization.
Visual LOD is persistent data whose representation is selected per view by
[`VisualLodPolicy`](../lib/src/scene/visual.rs) and submitted by
[`VisualLodRenderer`](../lib/src/scene/visual.rs)
([#667](https://github.com/ramate-io/maybraid/issues/667)).
Forest tiles store a [`VisualInstance`](../lib/src/scene/visual.rs) list on a
[`VisualLodRoot`](../lib/src/scene/visual.rs) sibling. Geometry is cached by
[`SceneRef`](../../scene-ref) → [`ScenePrototype`](../../scene-ref); material by
[`MaterialRef`](../../material-ref). Policy picks a band per view; the
[`InstancePbrRenderer`](../visual-pbr/src/instance_pbr.rs) buckets per grove,
then `(mesh, material)`, and submits instanced draws. Camera motion does not
cook posed grove meshes or spawn visual `SceneChunk`s. High kits stay on the
exclusive semantic drain. [`VisualOwnsAppearance`](../lib/src/scene/visual.rs)
mutes non-High fulfill on that tile.

## API

Existing:

```rust
fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static;
```

Optional override:

```rust
fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk;
```

Default: `SceneChunk::primitive(self.scene_with_level(...))` — one spawn unit (full scene build still happens up front unless overridden).

## Lifecycle

1. Sync inserts `LodLevelSpawnRequest` when the desired root is missing.
2. **Begin** creates a hidden `LodLevelRoot` + `LodLevelRootPending` and flattens the chunk tree into `LodChunkFulfillment`.
3. **Drain** is an exclusive `&mut World` system: `World::spawn_scene` under
   [`LodChunkFulfillBudget::spawn_weights_per_frame`](../lib/src/scene/refresh/sync/chunk/types.rs)
   **and** [`spawn_time_per_frame`](../lib/src/scene/refresh/sync/chunk/types.rs)
   (`min(weight, elapsed)`). Stops before the next `pull_primitive` when time is up.
4. **Complete** removes pending, sets the root `Inherited`, hides sibling roots.
5. Cull/GC may despawn non-desired ready roots as today; the desired level (including in-progress pending) is never culled.

Cancel: if the desired level changes, pending roots for other levels are despawned and their queues dropped.

## Registration

```rust
add_lod_refresh_chunk_full_for::<MyHost>(app); // update + chunk fulfill + cull
// or
add_lod_refresh_chunk_for::<MyHost>(app);      // fulfill only (probe / region writes level)
// or Avian region stack (see chico sbs-trees-playground `vegetation_lod.rs`)
```

Pending hosts use [`SemanticLodScene::host`](../lib/src/scene/lod_scene.rs) (core pending
shell + [`host_contents`](../lib/src/scene/lod_scene.rs)); chunk fulfill streams
[`scene_chunks_with_level`](../lib/src/scene/lod_scene.rs). Domain types override
`host_contents` only — do not re-stamp `lod_host_scene_pending`.

## Future: coalescing and compaction

Many tiny weights can dominate scheduling overhead. Planned follow-ups:

- **Coalescing** — merge adjacent cheap primitives (or subtrees under a weight floor) into one spawn unit before enqueue.
- **Compaction** — rewrite a deep `SubChunks` tree into a flatter primitive list so the drain loop stays cheap.

Lazy materialization of subtrees (factories evaluated only when the scheduler reaches them) is also intentionally out of scope for the first cut.

## Related

- [`LodChunk` / `SceneChunk`](../lib/src/scene/chunk.rs)
- [`VisualLodScene`](../lib/src/scene/visual.rs)
- [`chunk_fulfill`](../lib/src/chunk_fulfill.rs)
- [Richmond CONTRIBUTING — LodScene](../../richmond/CONTRIBUTING.md#lodscene-on-buildings)
