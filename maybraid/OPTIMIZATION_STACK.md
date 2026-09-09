# Optimization stack (LOD produce / generate store)

Tracy: [`urbanization-lod-late.tracy`](../urbanization-lod-late.tracy) (10:22, late **16 FPS**, ~20 GB RSS). Produce (refresh) and generate store (load) are different axes. Do not gate one on the other.

Issues: [#792](https://github.com/ramate-io/maybraid/issues/792) · [#793](https://github.com/ramate-io/maybraid/issues/793) · [#794](https://github.com/ramate-io/maybraid/issues/794) · [#795](https://github.com/ramate-io/maybraid/issues/795). Related assembly / pad budget: [#720](https://github.com/ramate-io/maybraid/issues/720). RC: [#71](https://github.com/ramate-io/maybraid/issues/71).

**Bullseye is refresh, not load.** Generate keep (3 km) and present keep (1 km) decide what exists. `Bullseye.outer` is the cube **edge** of who gets a `LodSceneLevel` this impulse. World `(50, 2000)` is ±1 km.

```text
A  792 stopgap     →  C  795 keyed cache  →  D  795/794 distinct P
                    ↘  B  792 200 m outer (optional measure)
E  793 store evict     (anytime after A)
F  793/720 pad compose (with #720 stacking)
```

| Slice | Issue | Status | Ships |
| --- | --- | --- | --- |
| **A** | [792](https://github.com/ramate-io/maybraid/issues/792) | **in tree** | No `LodSceneRefreshAabb` bus. Region plugins push [`LodProduceRegionSink`](lod/lib/src/scene/refresh/levels/produce.rs). One [`fill_lod_produce_cache`](lod/lib/src/scene/refresh/levels/produce.rs) (`Camera` **or** `LodViewer`). Mob High pulse writes typed `LodSceneRefreshRegion` only. Cache is still **one union**. |
| B | [792](https://github.com/ramate-io/maybraid/issues/792) | next measure | World `Bullseye { inner: 50, outer: 200 }` **after A**. Same long fly. Skip if going to D. |
| C | [795](https://github.com/ramate-io/maybraid/issues/795) | next code | `LodProduceCaches` keyed by `TypeId<M>`. Fill from typed regions. `LodRefreshChannel<M>` on `T`. Emit per channel. One `Bullseye` `P` still OK. |
| D | [795](https://github.com/ramate-io/maybraid/issues/795) / [794](https://github.com/ramate-io/maybraid/issues/794) | after C | `VegetationRefresh(50, 200)`, `UrbanizationRefresh(50, 400)`, `TerrainRefresh(100, 400)`. World stops clobbering buildings’ 500. Close 794. |
| E | [793](https://github.com/ramate-io/maybraid/issues/793) | parallel | `SpatialIndex` untrack outside keep+slack. Pad **budget** stays [#720](https://github.com/ramate-io/maybraid/issues/720). |
| F | [793](https://github.com/ramate-io/maybraid/issues/793) / [720](https://github.com/ramate-io/maybraid/issues/720) | with 720 | Budgeted pad compose; dirty-id enqueue, not whole `presentation_region()`. |

[#794](https://github.com/ramate-io/maybraid/issues/794) is the problem statement (shared `LodProduceCache` union). Close it when D lands.

Do **not** wait on [#720](https://github.com/ramate-io/maybraid/issues/720) type stacking to do A–D. `M` is the stable key; `UrbanizedTerrain<Durham>` registers onto `TerrainBullseye` later. Cull stays one [`LodCullProduceCache`](lod/lib/src/scene/refresh/cull_regions/cache.rs) until it shows up on Tracy.

## A (this change)

Standing late frames were filling every tick because [`pulse_world_mob_high_lod`](world/lib/src/mobs.rs) wrote a 450 m cube onto the untyped AABB bus. Vegetation and mobs each added `LodSceneRefreshLevelsFillPlugin<I, F>` (`Camera` vs `LodViewer`) and queried Host twice.

After A: produce runs when a region plugin actually emits; one Host query; mob pulse cannot widen the union. Walking still unions whatever region plugins push (including `MobHighLodRegion` on translation, and world `Bullseye` 2 km on a 50 m cell-cross). That is C/D.
