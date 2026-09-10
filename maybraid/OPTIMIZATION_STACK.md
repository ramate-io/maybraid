# Optimization stack (LOD produce / generate store)

Tracy: [`urbanization-lod-late.tracy`](../urbanization-lod-late.tracy) (baseline) · [`pre-alpha-optimization-stack.tracy`](../pre-alpha-optimization-stack.tracy) (A) · [`pre-alpha-optimization-stack-002.tracy`](../pre-alpha-optimization-stack-002.tracy) (B, 11:06) · [`pre-alpha-optimization-stack-003.tracy`](../pre-alpha-optimization-stack-003.tracy) (C, 7:33). Produce (refresh) and generate store (load) are different axes. Do not gate one on the other.

Issues: [#792](https://github.com/ramate-io/maybraid/issues/792) · [#793](https://github.com/ramate-io/maybraid/issues/793) · [#794](https://github.com/ramate-io/maybraid/issues/794) · [#795](https://github.com/ramate-io/maybraid/issues/795). Related assembly / pad budget: [#720](https://github.com/ramate-io/maybraid/issues/720). Hitch valve (not mean FPS): [#748](https://github.com/ramate-io/maybraid/issues/748). RC: [#71](https://github.com/ramate-io/maybraid/issues/71).

**Bullseye is refresh, not load.** Generate keep (3 km) and present keep (1 km) decide what exists. `Bullseye.outer` is the cube **edge** of who gets a `LodSceneLevel` this impulse. World `(50, 200)` is ±100 m. Produce is per `TypeId<M>`; mob High no longer scores vegetation hits.

Fill still scales with **Avian `Host` collider count**. Live woody High/Medium used to spawn one `FlattenedPlant` host per tree under `ChicoGroveHost`. This change emits posed kits on the **grove host only**. Isolated `/show` plants stay flattened hosts.

```text
A  792 stopgap     →  C  795 keyed cache  →  D  795/794 distinct P (veg outer ~1.4–2 km)
                    ↘  B  792 200 m outer (measured; do not shrink the world again)
grove host (this)      (measure fill / entity count vs C)
E  793 store evict     (anytime after A)
F  720 pad compose     (hitches; pair #748)
G  Host layers by M    (the remaining fill wall)
```

| Slice | Issue | Status | Ships |
| --- | --- | --- | --- |
| **A** | [792](https://github.com/ramate-io/maybraid/issues/792) | **in tree** | No `LodSceneRefreshAabb` bus. One [`fill_lod_produce_cache`](lod/lib/src/scene/refresh/levels/produce.rs) (`Camera` **or** `LodViewer`). |
| **B** | [792](https://github.com/ramate-io/maybraid/issues/792) | **measured** | World `Bullseye { inner: 50, outer: 200 }`. Late fill still ~12 ms / **~16 FPS**. Skip more B flights. |
| **C** | [795](https://github.com/ramate-io/maybraid/issues/795) | **measured** | [`LodProduceCaches`](lod/lib/src/scene/refresh/levels/produce.rs) keyed by `TypeId<M>`. Emit collapsed (2–6 ms → 0.02–0.2 ms). Fill still 5.8–7.6 ms (shared Host layer). Veg upgrades delayed: B’s 200 m outer + C removing the mob 450 m crutch = Low blobs until ~100 m. |
| **grove host** | — | **this change** | Woody High/Medium kits spawn under `ChicoGroveHost` (no per-tree `FlattenedPlant` hosts). Stick capsules move onto the grove host. Expect fill / physics / vis to track tile count, not tree count. |
| **D** | [795](https://github.com/ramate-io/maybraid/issues/795) / [794](https://github.com/ramate-io/maybraid/issues/794) | after grove host | Distinct `P`: vegetation outer **~1.4–2 km edge** if present keep stays 1 km (Medium at 350–700 m). Urban 400, terrain 400. Restores approach upgrades C lost. Does **not** cut the 7 ms Host fill. Close 794. Do not ship D as `(50, 200)` — that keeps the blobs. |
| **E** | [793](https://github.com/ramate-io/maybraid/issues/793) | parallel | `SpatialIndex` untrack outside keep+slack. RSS / fewer far entities. Mild present-cull / physics. Pad **budget** stays [#720](https://github.com/ramate-io/maybraid/issues/720). |
| **F** | [720](https://github.com/ramate-io/maybraid/issues/720) | with 720 | Budgeted pad compose (dirty-id enqueue, not whole `presentation_region()`). Cuts 200–500 ms pad spikes. Pair [`#748`](https://github.com/ramate-io/maybraid/issues/748) (`Time<Virtual>::set_max_delta` ~33 ms) as a hitch valve, not a mean-FPS cut. |
| **G** | [795](https://github.com/ramate-io/maybraid/issues/795) | after D | Per-channel Host layers (`HostVegetation` / `HostUrban` / `HostTerrain` / `HostMob`) + `SpatialQueryFilter` by `M`. Optional: cell/timer-gate `MobHighLodRegion` (today Spotlight-every-translation ±450 m). This is the remaining produce fill wall. |

[#794](https://github.com/ramate-io/maybraid/issues/794) is the problem statement (shared `LodProduceCache` union). Close it when D lands.

Do **not** wait on [#720](https://github.com/ramate-io/maybraid/issues/720) type stacking to do A–D. `M` is the stable key; `UrbanizedTerrain<Durham>` registers onto `TerrainBullseye` later. Cull stays one [`LodCullProduceCache`](lod/lib/src/scene/refresh/cull_regions/cache.rs) until it shows up on Tracy.

Do not shrink the whole world first. Keep C. Do not fly more B.

## A (landed)

Standing late frames were filling every tick because [`pulse_world_mob_high_lod`](world/lib/src/mobs.rs) wrote a 450 m cube onto the untyped AABB bus. Vegetation and mobs each added `LodSceneRefreshLevelsFillPlugin<I, F>` (`Camera` vs `LodViewer`) and queried Host twice.

After A: produce runs when a region plugin actually emits; one Host query; mob pulse cannot widen the union. Walking still unions whatever region plugins push (including `MobHighLodRegion` on translation).

## B (measured)

World `Bullseye.outer` is 200 m (cube edge). [`pre-alpha-optimization-stack-002.tracy`](../pre-alpha-optimization-stack-002.tracy) (11:06, 30,685 frames): fill still **once per frame**. At 10 min, fill 11.5 ms (p95 12.4, max 24) vs A’s 12.7 / 12.9 / 25. Emit 5.8 ms. Late FPS **16.7 → 15.5**. Mid-session fill/FPS were healthier (cell-cross no longer 2 km); the every-translation 450 m Host query was the remaining produce wall.

## C (measured)

[`LodProduceRegionSink`](lod/lib/src/scene/refresh/levels/produce.rs) is keyed by `TypeId<M>`. [`LodSceneRefreshLevelsPlugin<T, M>`](lod/lib/src/scene/refresh/levels/produce.rs) stamps [`LodRefreshChannel<M>`](lod/lib/src/scene/refresh/levels/produce.rs). Fill queries each channel AABB, then keeps only members. Emit walks those hits. Dual bullseye + spotlight on the same `T` is two markers, max fold unchanged.

[`pre-alpha-optimization-stack-003.tracy`](../pre-alpha-optimization-stack-003.tracy) (7:33): emit 2–6 ms → 0.02–0.2 ms. Fill still every frame, late **5.8–7.6 ms** (sometimes worse than B: N channel queries + 450 m mob query still returns all Host cuboids, then skips trees). Vegetation upgrades delayed — spawn still stamps the right band; approach no longer gets the 450 m veg refresh. B’s 200 m veg outer + C removing that crutch = Low blobs until ~100 m.

## Grove host (this change)

`FoliageNode` / `StickNode` leftovers are not the live forest path. Woody High/Medium was `ChicoGroveHost` → `FlattenedPlant` × N. Fill, physics, and visibility scaled with per-tree hosts. Triangle budget was already fine.

Now High/Medium emit [`flattened_vegetation_scene_chunks`](chico/vegetation-components/src/lib.rs) under the grove (same family as Low canopy proxies). Isolated `/show` still uses `FlattenedComponentsOnly`. Playable stick capsules stamp on the grove host (High-IR, one compound per tile) so walking does not lose trunks.

Measure vs C: Host query size, entity count, late fill, vegetation upgrade (still gated on D’s veg outer).

## D (next for looks)

Distinct `P` so vegetation produce covers the 1 km present ring. Suggested: `VegetationRefresh` outer **1400–2000** (cube edge), `UrbanizationRefresh(50, 400)`, `TerrainRefresh(100, 400)`. World stops clobbering buildings’ 500. Close 794. This restores Medium at 350–700 m; it does not cheapen the shared Host query.

## E (RSS)

Evict generate-store entries outside keep+slack. Fewer far entities; mild present-cull / physics. Not the late fill cliff.

## F (hitches)

Budgeted pad compose ([#720](https://github.com/ramate-io/maybraid/issues/720)). Dirty-id enqueue, not whole `presentation_region()`. Pair [#748](https://github.com/ramate-io/maybraid/issues/748) so a 286 ms pad spike cannot stretch `Time<Virtual>` unboundedly.

## G (remaining fill)

Avian still queries one `PhysicsInteractionLayer::Host` mask per channel AABB. Split Host by channel (`HostVegetation` / `HostUrban` / `HostTerrain` / `HostMob`) and filter `SpatialQueryFilter` by `M`. Optional: do not rebuild `MobHighLodRegion` on every Spotlight translation.
