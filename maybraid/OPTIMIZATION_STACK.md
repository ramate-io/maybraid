# Optimization stack (LOD produce / generate store)

Tracy: [`urbanization-lod-late.tracy`](../urbanization-lod-late.tracy) (baseline) · [`pre-alpha-optimization-stack.tracy`](../pre-alpha-optimization-stack.tracy) (A) · [`pre-alpha-optimization-stack-002.tracy`](../pre-alpha-optimization-stack-002.tracy) (B, 11:06). Produce (refresh) and generate store (load) are different axes. Do not gate one on the other.

Issues: [#792](https://github.com/ramate-io/maybraid/issues/792) · [#793](https://github.com/ramate-io/maybraid/issues/793) · [#794](https://github.com/ramate-io/maybraid/issues/794) · [#795](https://github.com/ramate-io/maybraid/issues/795). Related assembly / pad budget: [#720](https://github.com/ramate-io/maybraid/issues/720). RC: [#71](https://github.com/ramate-io/maybraid/issues/71).

**Bullseye is refresh, not load.** Generate keep (3 km) and present keep (1 km) decide what exists. `Bullseye.outer` is the cube **edge** of who gets a `LodSceneLevel` this impulse. World `(50, 200)` is ±100 m. Produce is per `TypeId<M>`; mob High no longer scores vegetation hits.

```text
A  792 stopgap     →  C  795 keyed cache  →  D  795/794 distinct P
                    ↘  B  792 200 m outer (optional measure)
E  793 store evict     (anytime after A)
F  793/720 pad compose (with #720 stacking)
```

| Slice | Issue | Status | Ships |
| --- | --- | --- | --- |
| **A** | [792](https://github.com/ramate-io/maybraid/issues/792) | **in tree** | No `LodSceneRefreshAabb` bus. One [`fill_lod_produce_cache`](lod/lib/src/scene/refresh/levels/produce.rs) (`Camera` **or** `LodViewer`). |
| **B** | [792](https://github.com/ramate-io/maybraid/issues/792) | **measured** | World `Bullseye { inner: 50, outer: 200 }`. Late fill still ~12 ms / **~16 FPS**. Mid-session a bit cheaper; walk wall is still `MobHighLodRegion` (±450 m) every translation. Skip more B flights. |
| **C** | [795](https://github.com/ramate-io/maybraid/issues/795) | **in tree** | [`LodProduceCaches`](lod/lib/src/scene/refresh/levels/produce.rs) keyed by `TypeId<M>`. [`LodRefreshChannel<M>`](lod/lib/src/scene/refresh/levels/produce.rs) on `T`. Emit per channel. Avian still queries each channel AABB (no new layers). One `Bullseye` `P` still OK. Same long fly to measure. |
| D | [795](https://github.com/ramate-io/maybraid/issues/795) / [794](https://github.com/ramate-io/maybraid/issues/794) | after C | `VegetationRefresh(50, 200)`, `UrbanizationRefresh(50, 400)`, `TerrainRefresh(100, 400)`. World stops clobbering buildings’ 500. Close 794. |
| E | [793](https://github.com/ramate-io/maybraid/issues/793) | parallel | `SpatialIndex` untrack outside keep+slack. Pad **budget** stays [#720](https://github.com/ramate-io/maybraid/issues/720). |
| F | [793](https://github.com/ramate-io/maybraid/issues/793) / [720](https://github.com/ramate-io/maybraid/issues/720) | with 720 | Budgeted pad compose; dirty-id enqueue, not whole `presentation_region()`. |

[#794](https://github.com/ramate-io/maybraid/issues/794) is the problem statement (shared `LodProduceCache` union). Close it when D lands.

Do **not** wait on [#720](https://github.com/ramate-io/maybraid/issues/720) type stacking to do A–D. `M` is the stable key; `UrbanizedTerrain<Durham>` registers onto `TerrainBullseye` later. Cull stays one [`LodCullProduceCache`](lod/lib/src/scene/refresh/cull_regions/cache.rs) until it shows up on Tracy.

## A (this change)

Standing late frames were filling every tick because [`pulse_world_mob_high_lod`](world/lib/src/mobs.rs) wrote a 450 m cube onto the untyped AABB bus. Vegetation and mobs each added `LodSceneRefreshLevelsFillPlugin<I, F>` (`Camera` vs `LodViewer`) and queried Host twice.

After A: produce runs when a region plugin actually emits; one Host query; mob pulse cannot widen the union. Walking still unions whatever region plugins push (including `MobHighLodRegion` on translation).

## B (this change)

World `Bullseye.outer` is 200 m (cube edge). [`pre-alpha-optimization-stack-002.tracy`](../pre-alpha-optimization-stack-002.tracy) (11:06, 30,685 frames): fill still **once per frame**. At 10 min, fill 11.5 ms (p95 12.4, max 24) vs A’s 12.7 / 12.9 / 25. Emit 5.8 ms. Late FPS **16.7 → 15.5**. Mid-session fill/FPS were healthier (cell-cross no longer 2 km); the every-translation 450 m Host query was the remaining produce wall.

## C (this change)

[`LodProduceRegionSink`](lod/lib/src/scene/refresh/levels/produce.rs) is keyed by `TypeId<M>`. [`LodSceneRefreshLevelsPlugin<T, M>`](lod/lib/src/scene/refresh/levels/produce.rs) stamps [`LodRefreshChannel<M>`](lod/lib/src/scene/refresh/levels/produce.rs). Fill queries each channel AABB, then keeps only members. Emit walks those hits. Dual bullseye + spotlight on the same `T` is two markers, max fold unchanged.

Walk-frame mob High still *queries* ±450 m (Avian Host layer is shared) but no longer writes `LodSceneRefreshLevel` for groves/buildings in that cube. Expect emit and level-fold to drop; fill may stay expensive until D or a later typed index. One `Bullseye` resource still means vegetation and buildings emit the same 200 m cube — D splits `P`.
