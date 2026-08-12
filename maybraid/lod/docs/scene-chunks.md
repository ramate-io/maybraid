# Incremental LOD scene chunks

Amortize spawning a level root across frames under a weight budget.

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
3. **Drain** spawns primitives under [`LodChunkFulfillBudget::weights_per_frame`](../lib/src/scene/chunk_fulfill.rs).
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

Pending hosts use [`lod_host_scene_pending`](../lib/src/scene/host.rs) / [`LodScene::host`](../lib/src/scene/lod_scene.rs); chunk fulfill streams [`scene_chunks_with_level`](../lib/src/scene/lod_scene.rs).

## Future: coalescing and compaction

Many tiny weights can dominate scheduling overhead. Planned follow-ups:

- **Coalescing** — merge adjacent cheap primitives (or subtrees under a weight floor) into one spawn unit before enqueue.
- **Compaction** — rewrite a deep `SubChunks` tree into a flatter primitive list so the drain loop stays cheap.

Lazy materialization of subtrees (factories evaluated only when the scheduler reaches them) is also intentionally out of scope for the first cut.

## Related

- [`SceneChunk`](../lib/src/scene_chunk.rs)
- [`chunk_fulfill`](../lib/src/chunk_fulfill.rs)
- [Richmond CONTRIBUTING — LodScene](../../richmond/CONTRIBUTING.md#lodscene-on-buildings)
