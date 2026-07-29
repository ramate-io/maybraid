# Contributing to Richmond

Richmond is the cellular urbanization stack: footprints, rooms, ornamental kit,
and building composition. This document covers how **buildings** should author
and present geometry on top of [`building-components`](building-components/).

## Crates

| Crate | Role |
|-------|------|
| [`building-components`](building-components/) | Domain IR + kit assets. Authoring types are `*Node` values (`FloorNode`, `PartitionNode`, `StairNode`, `DoorNode`, `RoofNode`): **style + geometry + placement** (+ optional [`ParentConfines`](building-components/src/parent_confines.rs)). Each node implements [`LodScene`](../lod/lib/src/gen/presentation.rs). Tessellation into kit pieces is private to the domain. Partition IR is primitive (no portals). |
| [`buildings`](buildings/) | Building procedures. Compose constraints, layouts, and helpers (`Walling` / `ArcWall` / `LinearWall` / `PolylineWall`, `ArcSpire`, …) into **owned collections of nodes** (and rare non-mesh features such as lights). Implement `LodScene` by emitting those nodes (and helpers) under the requested LOD. |
| [`buildings-playground`](buildings-playground/) | Preview / CLI. Spawns hosts once; LOD flips update [`LodSceneLevel`](../lod/lib/src/lod_level.rs) in place (no whole-tree despawn). |

Shared pose helpers (`pose`, `posed_glb`, `with_pose`, `scene_children`) live in building-components and should not be reimplemented per building.

## Authoring model

Buildings **emit authored domain types**; they do not tessellate kit pieces or call style→GLB mapping themselves.

```text
constraints / layout helpers
        │
        ▼
  Vec<FloorNode>, Vec<PartitionNode>, …
        │
        ▼
  scene_lod_level / scene_lod_status
  scene_with_level / lod_host_scene
```

Preferred shape for a storey or room:

```rust
pub struct ExampleFloor {
    pub floors: Vec<FloorNode>,
    pub partitions: Vec<PartitionNode>,
    pub stairs: Vec<StairNode>,
}

impl ExampleFloor {
    pub fn new(/* constraints, noise, … */) -> Self {
        // Layout → construct nodes with Style + Geometry + Placement.
        Self { /* … */ }
    }
}
```

Helpers such as [`Walling`](buildings/src/walling.rs) (`ArcWall` / `LinearWall` / `PolylineWall`) / `ArcSpire` are fine when they **produce** `Vec<PartitionNode>` / `StairNode`. Use **partition** for primitive kit IR and **wall** for portal-sensitive path helpers.

## Allocate cells, fill in children

Higher-order room types (e.g. [`Bedroom`](buildings/src/bedroom.rs)) own **layout**: they `subset` child AABBs from [`CellConstraints`](buildings/src/constraints.rs) and construct lower-order types. Lower-order types own **fill**.

Constructors take the child's [`CellConstraints`](buildings/src/constraints.rs). Do not pass a parent `&CellConstraints` “for context”.

Room layout is **noise-fitted**: [`BedroomLayout::fit`](buildings/src/bedroom/layout.rs) with [`BedroomFillParams`](buildings/src/bedroom/layout.rs) (`spaciousness`, `occupancy`). Circulation exclusions and internal door-swing rules apply as before.

## `LodScene` on buildings

- `scene_lod_level` — desired [`LodSceneLevel`](../lod/lib/src/lod_level.rs) (cheap).
- `scene_lod_status` — `Unchanged` or `Changed(level)`.
- `scene_with_level` — primary builder for one level root.
- `scene_with_lod` — first present via [`lod_host_scene`](../lod/lib/src/lod_scene_host.rs).

Hosts flip level-root visibility / lazily spawn missing roots. Nested hosts are independent.

**Fine pass:** [`LodFinePassPlugin`](../lod/lib/src/fine_pass.rs) tracks any [`LodViewer`](../lod/lib/src/fine_pass.rs) into [`LodViewerState`](../lod/lib/src/fine_pass.rs), then `add_fine_pass_for::<T>()` updates levels and fulfills [`LodLevelSpawnRequest`](../lod/lib/src/lod_scene_host.rs) via ephemeral [`LodRef`](../lod/lib/src/lod_ref.rs) + [`LodHostBounds`](../lod/lib/src/fine_pass.rs). Cameras are playground-only (`LodViewer` on the fly-cam). See also [structural LOD collectors](../lod/docs/structural-lod-collectors.md).

### `ParentConfines` (building-components only)

[`ParentConfines`](building-components/src/parent_confines.rs) is an **IR field** on nodes — not part of general `lod`:

- `External` — façade / silhouette candidates; normal distance/extent mesh banding.
- `Internal(InternalShape)` — detail gated by [`INTERNAL_REVEAL_FACTOR`](building-components/src/parent_confines.rs) (`5`) × radius:
  - [`InternalShape::Ball`](building-components/src/parent_confines.rs) — **floor- or room-compartment** ball. Prefer authoring at this grain.
  - [`InternalShape::Capsule`](building-components/src/parent_confines.rs) — long non-compartmentalized volumes (e.g. one continuous vertical spire); distance to the medial segment.

Do **not** hang one Internal ball on an entire multi-storey building. Pass the compartment footprint — do not pre-multiply by the reveal factor.

### Wizard’s Tower levels

LOD uses **capsule surface distance** (meters outside a vertical footprint capsule through the full tower AABB). Tall height no longer inflates the Low cutoff:

| Level | Content | Distance |
|-------|---------|----------|
| High | Exterior + per-storey internals + spire capsule | ≤ 5 × footprint radius |
| Medium | Exterior walls only | ≤ [`LOW_RES_CUTOFF_METERS`](buildings/src/wizards_tower/tower_lod.rs) (raw world meters) |
| Low | Cylinder silhouette | beyond |

Scale-dependent [`ParentConfines`](building-components/src/parent_confines.rs) may still reveal internals inside High even when that radius reaches farther than a short capsule-based feel — that clash is acceptable.

### Partition mesh resolution

Warm high/mid/low SceneRef roots under a **single** partition-node host. Parent banding uses linear factors (`distance / max_extent`):

| Band | Factor (linear / polyline parent) |
|------|--------|
| High | ≤ [`LINEAR_HIGH_FACTOR`](building-components/src/partitions/geometry/linear.rs) (5) |
| Medium | ≤ [`LINEAR_MEDIUM_FACTOR`](building-components/src/partitions/geometry/linear.rs) (20) |
| Low | ≤ [`LINEAR_LOW_FACTOR`](building-components/src/partitions/geometry/linear.rs) (500) |
| UltraLow | elsewhere (shares low mesh for now) |

**Polyline** is a short-run primitive: one LOD parent for the whole run (kits are content, not nested hosts). Prefer splitting long paths in walling/buildings. Joint kits under a polyline follow the parent level (high/mid GLBs only; omitted at Low). Lone joint leaf banding uses tighter factors (High ≤ 3, Medium ≤ 12).

### Roof mesh resolution

Roof kits use the same distance / extent probe shape ([`RoofLodProbe`](building-components/src/roofs/lod.rs)) but **tighter** High / Medium thresholds than walls:

| Band | Factor |
|------|--------|
| High | ≤ [`ROOF_HIGH_FACTOR`](building-components/src/roofs/lod.rs) (2.5) |
| Medium | ≤ [`ROOF_MEDIUM_FACTOR`](building-components/src/roofs/lod.rs) (10) |
| Low | ≤ [`ROOF_LOW_FACTOR`](building-components/src/roofs/lod.rs) (500) |
| UltraLow | elsewhere (shares low mesh for now) |

Shared mapping lives in [`lod_band`](building-components/src/lod_band.rs); fine-phase updates run `update_partition_host_levels` and `update_roof_host_levels` separately.

## Internal vs external emission

At **High**, each floor/room emits its own Internal ball for compartment geometry. Continuous vertical features (the tower spire) share one [`ParentConfines::capsule`](building-components/src/parent_confines.rs) (`Internal(Capsule)`) so higher storeys do not pop in awkwardly while you are inside the shaft. **Medium** omits internals from the scene.

```rust
fn emit_internal_features(
    &self,
    children: &mut Vec<Box<dyn Scene>>,
    lod_ref: &LodRef,
) {
    let confines = ParentConfines::internal(
        self.storey_confine_center(),
        self.storey_confine_radius(),
    );
    for floor in &self.floors {
        children.push(Box::new(
            floor.clone().with_confines(confines).scene_with_lod(lod_ref),
        ));
    }
}
```

## What not to do

- Do not put `ParentConfines` in the general `lod` crate.
- Do not despawn a whole building host on LOD flips — update `LodSceneLevel`.
- Do not treat “camera moved” as `Changed` by itself.

## Related reading

- [building-components README](building-components/README.md)
- [buildings README](buildings/README.md) (urban kit taxonomy for higher-order authorship)
- [Urban art README](../art/urban/README.md)
- [`LodScene`](../lod/lib/src/gen/presentation.rs)
- [Structural LOD collectors](../lod/docs/structural-lod-collectors.md)
- [Maybraid contributing: `-models` crates](../CONTRIBUTING.md#-models-crates)
