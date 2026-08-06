# Contributing to Richmond

Richmond is the cellular urbanization stack: footprints, rooms, ornamental kit,
and building composition. This document covers how **buildings** should author
and present geometry on top of [`building-components`](building-components/).

## Crates

| Crate | Role |
|-------|------|
| [`building-components`](building-components/) | Domain IR + kit assets. Authoring types are `*Node` values (`FloorNode`, `PartitionNode`, `StairNode`, `DoorNode`, `RoofNode`, `PanelNode`, `FurnitureNode`, `LabelNode`): **style + geometry + placement** (+ optional [`ParentConfines`](building-components/src/parent_confines.rs); labels also carry a debug string). Each node implements [`LodScene`](../lod/lib/src/gen/presentation.rs). Tessellation into kit pieces is private to the domain. Partition IR is primitive (no portals). Labels render as colored wireframes; face text is a playground gizmo pass (scaled/wrapped to each face). |
| [`buildings`](buildings/) | Building procedures. Compose constraints, layouts, and helpers (`paneling` / `arcs` / `portals`, `ArcSpire`, …) into domain nodes via [`BuildingComponents`](building-components/src/lib.rs). Present component-only buildings as [`ComponentsOnly`](building-components/src/lib.rs)`<T>` for `LodScene`; keep a custom `LodScene` when hosts, silhouettes, or non-node extras are required. Playground joinery demos live under `wall_demo`. |
| [`buildings-playground`](buildings-playground/) | Preview / CLI. Spawns hosts once; LOD flips update [`LodSceneLevel`](../lod/lib/src/lod_level.rs) in place (no whole-tree despawn). |

Shared pose helpers (`pose`, `posed_glb`, `with_pose`, `scene_children`, `append_component_scenes`, `ComponentsOnly`) live in building-components and should not be reimplemented per building.

## Authoring model

Buildings **emit authored domain types**; they do not tessellate kit pieces or call style→GLB mapping themselves.

```text
constraints / layout helpers
        │
        ▼
  BuildingComponents (*_nodes_for_level)
        │
        ├── ComponentsOnly<T> → LodScene (component-only)
        └── custom LodScene (host / silhouette / lights)
              optionally via append_component_scenes
```

Preferred shape for a storey or room:

```rust
use richmond_building_components::{BuildingComponents, Layers};

pub struct ExampleFloor {
    pub floors: Vec<FloorNode>,
    pub partitions: Vec<PartitionNode>,
    pub stairs: Vec<StairNode>,
}

impl BuildingComponents for ExampleFloor {
    fn floor_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FloorNode> {
        Layers::from_free(self.floors.clone())
    }
    fn partition_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartitionNode> {
        Layers::from_free(self.partitions.clone())
    }
    fn stair_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StairNode> {
        Layers::from_free(self.stairs.clone())
    }
}

// Present: ComponentsOnly(&example).scene_with_lod(lod_ref)
```

Parents **merge** children’s layered node maps via [`Layers::extend`](building-components/src/layer.rs), they do not nest child `LodScene`s when both sides are component-only.

[`Layer`](building-components/src/layer.rs) is a **provenance** record (e.g. `"closet"`, `"envelope"`), not a stand-in for node type. Domain type stays on the trait method (`panel_nodes_for_level` vs `partition_nodes_for_level`). Higher-order types use layer names to decide what to do with that geometry; use [`Layers::free`](building-components/src/layer.rs) until a provenance label is useful.

Helpers such as [`portal_ring_wall`](buildings/src/arcs/portal_ring.rs) / paneling strips / `ArcSpire` are fine when they **produce** `PartitionNode` / `PanelNode` / `StairNode` via `BuildingComponents`. Use **partition** for arc/linear kit IR, **panel** for rectangle/triangle kits, and **portals** for opening assignment along a path. Door leaves stay empty until portal → `DoorNode` authorship exists.

## Allocate cells, fill in children

Higher-order room types own **layout**: they `subset` child AABBs from [`CellConstraints`](buildings/src/constraints.rs) and construct lower-order types. Lower-order types own **fill**.

Constructors take the child's [`CellConstraints`](buildings/src/constraints.rs). Do not pass a parent `&CellConstraints` “for context”.

Residential program fill now lives under
[`usage_areas`](buildings/src/usage_areas.rs) (`CommonBedroom`, livable quarters)
via the Fit / parameterized → plan path and the shared
[`placer`](buildings/src/placer.rs) KindSpec trier — not hierarchical
`CellConstraints` bedroom trees.

## `LodScene` on buildings

Most buildings should implement [`BuildingComponents`](building-components/src/lib.rs) and present via [`ComponentsOnly`](building-components/src/lib.rs)`<T>` (`scene_lod_status` = `Unchanged`; `scene_with_level` = [`component_only_scene`](building-components/src/lib.rs)).

Types with host banding, silhouettes, lights, or late-bound [`ParentConfines`](building-components/src/parent_confines.rs) (e.g. Wizard’s Tower) implement `BuildingComponents` and keep a custom `LodScene`. Prefer [`append_component_scenes`](building-components/src/lib.rs) for the node portion.

`LodScene` methods:

- `scene_lod_level` — desired [`LodSceneLevel`](../lod/lib/src/lod_level.rs) (cheap).
- `scene_lod_status` — `Unchanged` or `Changed(level)`.
- `scene_lod_culls` — inactive [`LodLevelRoot`](../lod/lib/src/lod_scene_host.rs)s this type is willing to **despawn** ([`LodSceneCulls`](../lod/lib/src/lod_cull.rs); default `None` keeps roots warm). Prefer helpers ([`cull_non_adjacent_bands`](../lod/lib/src/lod_cull.rs), [`cull_offset_bands`](../lod/lib/src/lod_cull.rs) / [`cull_bands_with_adjacent_depth`](../lod/lib/src/lod_cull.rs), [`cull_named_from_factor`](../lod/lib/src/lod_cull.rs)) over ad-hoc lists; “not current” alone is not a cull reason. Host GC never despawns the current level. After despawn, Sync + Fulfill bring the desired level back via `scene_with_level` (same as first spawn). **Do not casually cull the immediately adjacent band** — re-entering it forces an expensive respawn; prefer non-adjacent GC, or offset bands (halfway in) only when the adjacent root is heavy.
- `scene_with_level` — primary builder for one level root.
- `scene_with_lod` — first present via [`lod_host_scene`](../lod/lib/src/lod_scene_host.rs).

Hosts flip level-root visibility / lazily spawn missing roots. Nested hosts are independent.

**Fine pass:** [`LodFinePassPlugin`](../lod/lib/src/fine_pass.rs) tracks any [`LodViewer`](../lod/lib/src/fine_pass.rs) into [`LodViewerState`](../lod/lib/src/fine_pass.rs), then `add_fine_pass_for::<T>()` updates levels, fulfills [`LodLevelSpawnRequest`](../lod/lib/src/lod_scene_host.rs), and culls via ephemeral [`LodRef`](../lod/lib/src/lod_ref.rs) + [`LodHostBounds`](../lod/lib/src/fine_pass.rs). Probe-driven hosts use `add_fine_pass_cull_for::<T>()` alongside their own level updaters. Cameras are playground-only (`LodViewer` on the fly-cam). See also [structural LOD collectors](../lod/docs/structural-lod-collectors.md).

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

**Polyline** is a short-run primitive: one LOD parent for the whole run (kits are content, not nested hosts). Prefer splitting long paths in buildings/paneling. Joint kits under a polyline follow the parent level (high/mid GLBs only; omitted at Low). Lone joint leaf banding uses tighter factors (High ≤ 3, Medium ≤ 12).

### Roof mesh resolution

Roof kits use the same distance / extent probe shape ([`RoofLodProbe`](building-components/src/roofs/lod.rs)) but **tighter** High / Medium thresholds than walls:

| Band | Factor |
|------|--------|
| High | ≤ [`ROOF_HIGH_FACTOR`](building-components/src/roofs/lod.rs) (2.5) |
| Medium | ≤ [`ROOF_MEDIUM_FACTOR`](building-components/src/roofs/lod.rs) (10) |
| Low | ≤ [`ROOF_LOW_FACTOR`](building-components/src/roofs/lod.rs) (500) |
| UltraLow | elsewhere (shares low mesh for now) |

### Panel mesh resolution

Panel kits reuse the roof distance factors ([`PANEL_*_FACTOR`](building-components/src/panels/lod.rs)) but treat UltraLow as a **dedicated** host root: every style swaps to the shared flat low-res rectangle / right-triangle GLB (`urban/panels/flat/…_low_res.glb`).

| Band | Content |
|------|---------|
| High / Medium / Low | Style triad (`*_high_res` / `*_mid_res` / `*_low_res`) |
| UltraLow | Flat low-res ([`PANEL_ULTRA_LOW_RECTANGLE`](building-components/src/panels/lod.rs) / [`PANEL_ULTRA_LOW_RIGHT_TRIANGLE`](building-components/src/panels/lod.rs)) |

Shared mapping lives in [`lod_band`](building-components/src/lod_band.rs); fine-phase updates run `update_partition_host_levels`, `update_panel_host_levels`, and `update_roof_host_levels` separately.

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
- [buildings CONTRIBUTING](buildings/CONTRIBUTING.md) (Les Halles parameterized → plan → full / openings / usage areas)
- [Urban art README](../art/urban/README.md)
- [`LodScene`](../lod/lib/src/gen/presentation.rs)
- [Structural LOD collectors](../lod/docs/structural-lod-collectors.md)
- [Maybraid contributing: `-models` crates](../CONTRIBUTING.md#-models-crates)
