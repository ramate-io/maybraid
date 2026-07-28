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

- `External` — façade / silhouette candidates.
- `Internal { center, radius }` — ball enveloping a large open interior; [`apply_parent_confines`](building-components/src/parent_confines.rs) hides until the viewer is inside.

Use `.with_confines(...)` when emitting internals.

### Wizard’s Tower levels

| Level | Content |
|-------|---------|
| Low | Cylinder silhouette |
| Medium | Exterior walls only |
| High | Exterior + internals (`ParentConfines::Internal` on floor/stair/lantern nodes) |

### Partition mesh resolution

Warm high/mid/low MeshRef roots under partition hosts; [`PartitionLodProbe`](building-components/src/partitions/lod.rs) drives tier flips.

## Internal vs external emission

At **High**, set `ParentConfines::Internal` on internal nodes. **Medium** omits internals from the scene.

```rust
fn emit_internal_features(
    &self,
    children: &mut Vec<Box<dyn Scene>>,
    lod_ref: &LodRef,
    ball_center: Vec3,
    ball_radius: f32,
) {
    let confines = ParentConfines::internal(ball_center, ball_radius);
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
