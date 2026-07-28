# Contributing to Richmond

Richmond is the cellular urbanization stack: footprints, rooms, ornamental kit,
and building composition. This document covers how **buildings** should author
and present geometry on top of [`building-components`](building-components/).

## Crates

| Crate | Role |
|-------|------|
| [`building-components`](building-components/) | Domain IR + kit assets. Authoring types are `*Node` values (`FloorNode`, `WallNode`, `StairNode`, `DoorNode`, `RoofNode`): **style + geometry + placement** (+ optional [`ParentConfines`](building-components/src/parent_confines.rs)). Each node implements [`LodScene`](../lod/lib/src/gen/presentation.rs). Tessellation into kit pieces is private to the domain. |
| [`buildings`](buildings/) | Building procedures. Compose constraints, layouts, and helpers (`ArcWall`, `ArcSpire`, …) into **owned collections of nodes** (and rare non-mesh features such as lights). Implement `LodScene` by emitting those nodes (and helpers) under the requested LOD. |
| [`buildings-playground`](buildings-playground/) | Preview / CLI. Spawns hosts once; LOD flips update [`LodSceneLevel`](../lod/lib/src/lod_level.rs) in place (no whole-tree despawn). |

Shared pose helpers (`pose`, `posed_glb`, `with_pose`, `scene_children`) live in building-components and should not be reimplemented per building.

## Authoring model

Buildings **emit authored domain types**; they do not tessellate kit pieces or call style→GLB mapping themselves.

```text
constraints / layout helpers
        │
        ▼
  Vec<FloorNode>, Vec<WallNode>, …
        │
        ▼
  scene_lod_level / scene_lod_status
  scene_with_level / lod_host_scene
```

Preferred shape for a storey or room:

```rust
pub struct ExampleFloor {
    pub floors: Vec<FloorNode>,
    pub walls: Vec<WallNode>,
    pub stairs: Vec<StairNode>,
}

impl ExampleFloor {
    pub fn new(/* constraints, noise, … */) -> Self {
        // Layout → construct nodes with Style + Geometry + Placement.
        Self { /* … */ }
    }
}
```

Helpers such as `ArcWall` / `ArcSpire` are fine when they **produce** `Vec<WallNode>` / `StairNode`.

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

**Fine vs broad phase:** per-host checks are the fine phase. See [structural LOD collectors](../lod/docs/structural-lod-collectors.md).

### `ParentConfines` (building-components only)

[`ParentConfines`](building-components/src/parent_confines.rs) is an **IR field** on nodes — not part of general `lod`:

- `External` — façade / silhouette candidates; normal distance/extent mesh banding.
- `Internal { center, radius }` — **floor- or room-compartment** ball. Prefer authoring at this grain so a simple ball works. [`apply_parent_confines`](building-components/src/parent_confines.rs) hides until within [`INTERNAL_REVEAL_FACTOR`](building-components/src/parent_confines.rs) (`5`) × radius.
- `Capsule { a, b, radius }` — for long non-compartmentalized volumes (e.g. one continuous vertical spire). Distance is to the medial segment.

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

Warm high/mid/low MeshRef roots under partition hosts; [`PartitionLodProbe`](building-components/src/partitions/lod.rs) drives tier flips (`distance / max_extent`):

| Band | Factor |
|------|--------|
| High | ≤ 5 |
| Medium | ≤ 20 |
| Low | ≤ 500 |
| UltraLow | elsewhere (shares low mesh for now) |

## Internal vs external emission

At **High**, each floor/room emits its own Internal ball for compartment geometry. Continuous vertical features (the tower spire) share one [`ParentConfines::Capsule`](building-components/src/parent_confines.rs) so higher storeys do not pop in awkwardly while you are inside the shaft. **Medium** omits internals from the scene.

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
- [`LodScene`](../lod/lib/src/gen/presentation.rs)
- [Structural LOD collectors](../lod/docs/structural-lod-collectors.md)
- [Maybraid contributing: `-models` crates](../CONTRIBUTING.md#-models-crates)
