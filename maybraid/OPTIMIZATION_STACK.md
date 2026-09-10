# Optimization stack

Where CPU time still goes after the Gimme host split, and the order of work
that follows from Tracy — not a backlog of every possible render trick.

Related: [#800](https://github.com/ramate-io/maybraid/issues/800),
[#801](https://github.com/ramate-io/maybraid/issues/801),
[#802](https://github.com/ramate-io/maybraid/issues/802),
[#803](https://github.com/ramate-io/maybraid/issues/803).

## Already landed

| Step | What changed | Why it mattered |
| --- | --- | --- |
| Slim baseline / [#800](https://github.com/ramate-io/maybraid/issues/800) | Typed LOD refresh domains, less work in `Update` | Late-flight `Update` ~11.6 ms vs ~26.5 ms on `main` |
| [#803](https://github.com/ramate-io/maybraid/issues/803) | Flattened character visuals | Fewer nested visual hosts on the player |
| [#802](https://github.com/ramate-io/maybraid/issues/802) | `GimmeLodSceneHostIndex` for refresh / cull | Host cuboids left the Avian broadphase. Produce fill no longer climbs with collider count. |

Generate and present **ids** already live on typed [`SpatialIndex`](lod/lib/src/gen/spatial_index.rs)
resources (`ForestIndex`, urbanization, terrain, …). They were never the
Host-query death spiral. The unused Avian Generate / Present volume API is
deleted so those layers cannot be re-wired as Host-shaped colliders.

## What Tracy actually shows

Capture `pre-alpha-custom-spatial-index.tracy`: ~5 min, 13 058 frames, ~44 FPS
whole-flight. A ~140-frame late window (`rough_frames.csv`) is the better read
of “are we falling over.”

| Zone | Whole capture mean | Late ~140 frames |
| --- | ---: | ---: |
| `Update` | 11.7 ms | **10.2 ms** |
| `fill_lod_produce_cache` (`With<Camera>`) | 1.85 ms | **2.12 ms** |
| `fill_lod_produce_cache` (`With<LodViewer>`) | 1.41 ms | **1.66 ms** |
| `fill_lod_cull_produce_cache` | 1.58 ms | **1.89 ms** |
| `write_binned_instance_buffers<Opaque3d>` | 1.91 ms | **1.50 ms** |
| `check_visibility_cpu_culling` | 1.25 ms | **0.18 ms** |
| `PhysicsSchedule` | 1.14 ms (max 11 ms) | **0.12 ms** |
| `reindex_moved_hosts` | 0.18 ms | 0.19 ms |
| `sync_stick_colliders` | 0.077 ms | 0.062 ms |

Whole-capture visibility / collider / dirty-tree heat is load-in and hitch
**maxes**. The live budget is two produce fills + cull fill, then opaque
instance write, then present drains. Gimme reindex is cheap. Physics is quiet
once spawned.

`fill_lod_produce_cache` is **once per (`Index`, `F`)**. Vegetation refresh
uses `With<Camera>`; mobs use `With<LodViewer>`. Both collect `LodNode`
snapshots. Cull fill is a **separate** cache — folding produce does not
automatically fold cull.

## Next work (this order)

### 1. Hygiene — Avian Generate / Present volumes

**Status: done in this crate pass.**

[`lod-avian`](lod/avian/) no longer exposes `AvianLodGenerateIndex`,
`AvianLodPresentIndex`, or Generate / Present marshallers and physics layers.
`PatchSceneBounds` already stamped [`GimmeLodHostMarshaller`](lod/lib/src/scene/gimme.rs)
only; those Avian volumes were not live dummy colliders. This does **not**
move the ~10 ms `Update`. It removes the footgun of putting Host-shaped
cuboids back onto generate / present.

Leftover: `AvianLodSceneHostIndex` / `AvianLodSceneBoundsMarshaller` still exist
as an unused Avian Host query path. Refresh plugins do not install them.

### 2. One shared produce fill

One `fill_lod_produce_cache` (and one snapshot walk) for every refresh domain,
regardless of `With<Camera>` vs `With<LodViewer>`.

Late window: Camera fill ~2.1 ms + LodViewer fill ~1.7 ms. That is the next
measured `Update` win. Leave `fill_lod_cull_produce_cache` alone until produce
is shared; it is a different cache.

### 3. Flatten High / Medium building kits

[`building_scene_chunks`](richmond/building-components/src/lib.rs) emits
**each** panel, partition, floor, roof, joint, and (on High) stair / door /
furniture / label as its own nested [`LodScene`](lod/lib/src/scene/lod_scene.rs)
host.

A city block is hundreds of hosts and `Mesh3d`s inside the camera refresh AABB.
That is why urban is worse than forest High: plants already use
[`FlattenedComponentsOnly`](chico/vegetation-components/src/lib.rs) (one host,
posed kits). Buildings still pay the pre-flatten tax.

Mirror vegetation: **one building host**, kit instances (or
merged-and-**quantized** patches) underneath. Do not merge a unique whole-block
mesh unless that mesh is also shared.

This is the urban cardinality fix for produce hits, visibility, **and**
`write_binned_instance_buffers`. That zone scales with **visible `Mesh3d`
entities**, not triangle count. Quantizing plants (few archetypes, many poses)
cuts unique meshes; it does not cut instance-buffer writes. Folding many kit
entities into fewer meshes does.

### 4. Broader fewer-host work

Same campaign after buildings stop being a special case: leftover nested
`FoliageNode` / `StickNode` hosts, grove vs plant, Hidden warm roots that still
refresh. Keep shrinking what `fill_lod_produce_cache` and visibility walk.

## Not next (and why)

- **Moving generate onto Gimme for FPS.** Generate drains are tiny in the late
  window. Produce is already Gimme. The remaining fill cost is snapshot
  collection + host set size.
- **Caching stick compounds for frame time.** [`StickPhysicsAttached`](chico/forests/src/stick_physics.rs)
  already stamps once (budget 8 hosts/frame). Late `sync_stick_colliders` is
  ~60 µs. Hitch maxes are **presence** in Fixed (and parenting under a dirty
  `GlobalTransform`), not rebuild. Near-field physics is the lever, not a
  recipe cache.
- **Attacking instances by making unique merged patches.** That explodes GPU
  memory. Share first (quantize), then fold cardinality.

## How to re-measure

Prefer Tracy over hitch loggers.

1. `tracy-capture` a several-minute flight that includes urban.
2. Whole-capture CSV for maxes / load-in.
3. A late-window export (as `rough_frames.csv`) for the steady budget.
4. Compare `Update`, the three LOD fills, `write_binned` Opaque3d, and
   `reindex_moved_hosts`.
