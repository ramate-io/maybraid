# Incremental LOD scene chunks

Amortize spawning a **semantic** level root across frames under a weight budget.

[`SemanticLodScene`](../lib/src/scene/lod_scene.rs) (`LodScene` is an alias) builds
[`SceneChunk`](../lib/src/scene/chunk.rs) = `LodChunk<Box<dyn Scene>>`. Drain
spawns those scenes in the main World.

[`VisualLodScene`](../lib/src/scene/lod_scene.rs) builds
[`VisualSceneChunk`](../lib/src/scene/chunk.rs) = `LodChunk<VisualLodPrimitive>`.
That tree is **not** a Bevy `Scene` and is **not** consumed by
`drain_chunk_lod_fulfill`. [#667](https://github.com/ramate-io/maybraid/issues/667)
fills the visual leaf; this crate only defines the fork.

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

Visual (no consume plugin yet):

```rust
fn visual_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> VisualSceneChunk;
```

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
// or Avian region stack (see chico-forests `view.rs`)
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

- [`LodChunk` / `SceneChunk` / `VisualSceneChunk`](../lib/src/scene/chunk.rs)
- [`chunk_fulfill`](../lib/src/chunk_fulfill.rs)
- [Richmond CONTRIBUTING — LodScene](../../richmond/CONTRIBUTING.md#lodscene-on-buildings)
