# Optimization stack (LOD produce / generate store)

Tracy: [`urbanization-lod-late.tracy`](../urbanization-lod-late.tracy) (baseline) · [`pre-alpha-optimization-stack.tracy`](../pre-alpha-optimization-stack.tracy) (A) · [`pre-alpha-optimization-stack-002.tracy`](../pre-alpha-optimization-stack-002.tracy) (B, 11:06) · [`pre-alpha-optimization-stack-003.tracy`](../pre-alpha-optimization-stack-003.tracy) (C, 7:33) · [`pre-alpha-optimization-stack-004.tracy`](../pre-alpha-optimization-stack-004.tracy) (grove host, 15:09). Produce (refresh) and generate store (load) are different axes. Do not gate one on the other.

Issues: [#792](https://github.com/ramate-io/maybraid/issues/792) · [#793](https://github.com/ramate-io/maybraid/issues/793) · [#794](https://github.com/ramate-io/maybraid/issues/794) · [#795](https://github.com/ramate-io/maybraid/issues/795). Related assembly / pad budget: [#720](https://github.com/ramate-io/maybraid/issues/720). Hitch valve (not mean FPS): [#748](https://github.com/ramate-io/maybraid/issues/748). RC: [#71](https://github.com/ramate-io/maybraid/issues/71).

**Bullseye is refresh, not load.** Generate keep (3 km) and present keep (1 km) decide what exists. `Bullseye.outer` is the cube **edge** of who gets a `LodSceneLevel` this impulse. World `(50, 200)` is ±100 m. Produce is per `TypeId<M>`; mob High no longer scores vegetation hits.

Pay down to **60 FPS first**, then rebuild streaming behavior. Do not raise begin/drain to chase fill. Do not fly more B. **G is not the 30–40 FPS lever** — fill is already cheap after the grove host.

```text
A  792 stopgap     →  C  795 keyed cache  →  D  795/794 distinct P (veg outer ~1.4–2 km)
                    ↘  B  792 200 m outer (measured; do not shrink the world again)
grove host (004)       fill cheap; 100 m stick compound hitch
per-plant sticks       (this; tree-sized AABBs + vsync toggle)
E  793 store evict     (anytime after A)
F  720 pad compose     (hitches; pair #748)
quadrants              after 60 is stable (1 km present feel, not FPS)
G  Host layers by M    skip for FPS; optional later
```

| Slice | Issue | Status | Ships |
| --- | --- | --- | --- |
| **A** | [792](https://github.com/ramate-io/maybraid/issues/792) | **in tree** | No `LodSceneRefreshAabb` bus. One [`fill_lod_produce_cache`](lod/lib/src/scene/refresh/levels/produce.rs) (`Camera` **or** `LodViewer`). |
| **B** | [792](https://github.com/ramate-io/maybraid/issues/792) | **measured** | World `Bullseye { inner: 50, outer: 200 }`. Late fill still ~12 ms / **~16 FPS**. Skip more B flights. |
| **C** | [795](https://github.com/ramate-io/maybraid/issues/795) | **measured** | [`LodProduceCaches`](lod/lib/src/scene/refresh/levels/produce.rs) keyed by `TypeId<M>`. Emit collapsed (2–6 ms → 0.02–0.2 ms). Fill still 5.8–7.6 ms (shared Host layer). Veg upgrades delayed: B’s 200 m outer + C removing the mob 450 m crutch = Low blobs until ~100 m. |
| **grove host** | — | **measured (004)** | Woody High/Medium kits spawn under `ChicoGroveHost` (no per-tree `FlattenedPlant` hosts). Fill 0.26–0.91 ms. One 100 m stick compound per grove → `propagate_collider_transforms` p95 6–8 ms. Late Frame 33–36 ms is mostly vsync lock (~30 Hz). `RenderApp` 15–16 ms from t=0. |
| **per-plant sticks** | — | **this change** | Kits / produce stay on the grove host. One static compound per plant (`fixed_layers()`, not Host, not `LodScene`). Vsync toggle: `F8`, `/stats vsync`, `MAYBRAID_VSYNC=off`. |
| **D** | [795](https://github.com/ramate-io/maybraid/issues/795) / [794](https://github.com/ramate-io/maybraid/issues/794) | after 60 | Distinct `P`: vegetation outer **~1.4–2 km edge** if present keep stays 1 km (Medium at 350–700 m). Urban 400, terrain 400. Restores approach upgrades C lost. Close 794. Do not ship D as `(50, 200)` — that keeps the blobs. |
| **E** | [793](https://github.com/ramate-io/maybraid/issues/793) | parallel | `SpatialIndex` untrack outside keep+slack. RSS / fewer far entities. Mild present-cull / physics. Pad **budget** stays [#720](https://github.com/ramate-io/maybraid/issues/720). |
| **F** | [720](https://github.com/ramate-io/maybraid/issues/720) | with 720 | Budgeted pad compose (dirty-id enqueue, not whole `presentation_region()`). Cuts 200–500 ms pad spikes. Pair [`#748`](https://github.com/ramate-io/maybraid/issues/748) (`Time<Virtual>::set_max_delta` ~33 ms) as a hitch valve, not a mean-FPS cut. |
| **quadrants** | — | after 60 | Split a 100 m grove into four `LodScene` hosts so a corner High does not admit the whole orchard. Behavioral / streaming feel, not the 60 lever. |
| **G** | [795](https://github.com/ramate-io/maybraid/issues/795) | skip for FPS | Per-channel Host layers. Fill is already &lt;1 ms after grove host. |

[#794](https://github.com/ramate-io/maybraid/issues/794) is the problem statement (shared `LodProduceCache` union). Close it when D lands.

Do **not** wait on [#720](https://github.com/ramate-io/maybraid/issues/720) type stacking to do A–D. `M` is the stable key; `UrbanizedTerrain<Durham>` registers onto `TerrainBullseye` later. Cull stays one [`LodCullProduceCache`](lod/lib/src/scene/refresh/cull_regions/cache.rs) until it shows up on Tracy.

Do not shrink the whole world first. Keep C. Do not fly more B. Do not raise [`LodChunkFulfillBudget`](lod/lib/src/scene/refresh/sync/chunk/types.rs) time/weight until vsync-off shows headroom.

## A (landed)

Standing late frames were filling every tick because [`pulse_world_mob_high_lod`](world/lib/src/mobs.rs) wrote a 450 m cube onto the untyped AABB bus. Vegetation and mobs each added `LodSceneRefreshLevelsFillPlugin<I, F>` (`Camera` vs `LodViewer`) and queried Host twice.

After A: produce runs when a region plugin actually emits; one Host query; mob pulse cannot widen the union. Walking still unions whatever region plugins push (including `MobHighLodRegion` on translation).

## B (measured)

World `Bullseye.outer` is 200 m (cube edge). [`pre-alpha-optimization-stack-002.tracy`](../pre-alpha-optimization-stack-002.tracy) (11:06, 30,685 frames): fill still **once per frame**. At 10 min, fill 11.5 ms (p95 12.4, max 24) vs A’s 12.7 / 12.9 / 25. Emit 5.8 ms. Late FPS **16.7 → 15.5**. Mid-session fill/FPS were healthier (cell-cross no longer 2 km); the every-translation 450 m Host query was the remaining produce wall.

## C (measured)

[`LodProduceRegionSink`](lod/lib/src/scene/refresh/levels/produce.rs) is keyed by `TypeId<M>`. [`LodSceneRefreshLevelsPlugin<T, M>`](lod/lib/src/scene/refresh/levels/produce.rs) stamps [`LodRefreshChannel<M>`](lod/lib/src/scene/refresh/levels/produce.rs). Fill queries each channel AABB, then keeps only members. Emit walks those hits. Dual bullseye + spotlight on the same `T` is two markers, max fold unchanged.

[`pre-alpha-optimization-stack-003.tracy`](../pre-alpha-optimization-stack-003.tracy) (7:33): emit 2–6 ms → 0.02–0.2 ms. Fill still every frame, late **5.8–7.6 ms** (sometimes worse than B: N channel queries + 450 m mob query still returns all Host cuboids, then skips trees). Vegetation upgrades delayed — spawn still stamps the right band; approach no longer gets the 450 m veg refresh. B’s 200 m veg outer + C removing that crutch = Low blobs until ~100 m.

## Grove host (004)

`FoliageNode` / `StickNode` leftovers are not the live forest path. Woody High/Medium was `ChicoGroveHost` → `FlattenedPlant` × N. Fill, physics, and visibility scaled with per-tree hosts.

Now High/Medium emit [`flattened_vegetation_scene_chunks`](chico/vegetation-components/src/lib.rs) under the grove. Isolated `/show` still uses `FlattenedComponentsOnly`. [`pre-alpha-optimization-stack-004.tracy`](../pre-alpha-optimization-stack-004.tracy): `fill_lod_produce_cache` 0.26–0.91 ms (was 5.8–7.6). Emit ~6 µs. `RenderApp` 15.2–16.0 ms from t=0 (vsync wait). New CPU hitch: one ~100 m stick compound per grove (`propagate_collider_transforms` p95 6–8 ms, `mark_dirty_trees` p95 6 ms). Late Frame 33–36 ms → ~30 Hz vsync lock.

Loading still feels slow because a High tile is many kit chunks at weight 4, not one host at weight 1. Raising the 2 ms / weight cap admits more kits per frame — that fights 60 if you are already missing vsync.

## Per-plant sticks (this change)

Keep kits and produce on the grove host. Spawn one static compound per plant as ordinary children (`PhysicsInteractionLayer::fixed_layers()`). Not Host. Not `LodScene`. Produce fill stays cheap. Broadphase gets tree-sized AABBs again. Transform dirty still exists; Avian is not walking a 100 m compound.

Vsync: default `AutoVsync`. `F8`, `/stats vsync`, or `MAYBRAID_VSYNC=off` → `Immediate` for the next Tracy (confirm wait vs real work).

## D (looks, after 60)

Distinct `P` so vegetation produce covers the 1 km present ring. Suggested: `VegetationRefresh` outer **1400–2000** (cube edge), `UrbanizationRefresh(50, 400)`, `TerrainRefresh(100, 400)`. World stops clobbering buildings’ 500. Close 794. This restores Medium at 350–700 m.

## E (RSS)

Evict generate-store entries outside keep+slack. Fewer far entities; mild present-cull / physics. Not the late fill cliff.

## F (hitches)

Budgeted pad compose ([#720](https://github.com/ramate-io/maybraid/issues/720)). Dirty-id enqueue, not whole `presentation_region()`. Pair [#748](https://github.com/ramate-io/maybraid/issues/748) so a 286 ms pad spike cannot stretch `Time<Virtual>` unboundedly.

## Quadrants (after 60)

A 50 m `LodScene` finishes inside the 2 ms quantum; a 100 m orchard of kits does not. Standing on a corner, 3/4 of the old tile can stay Medium/Low. Foremost a **behavioral** lever so present 1 km *feels* like it streams — not a 60 FPS lever. Do not split until per-plant compounds are measured (otherwise 4× fat 50 m blobs).

## G (skip for FPS)

Avian still queries one `PhysicsInteractionLayer::Host` mask per channel AABB. Fill is already &lt;1 ms. Split Host by channel later if a channel AABB still returns the wrong cuboids.
