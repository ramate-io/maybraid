# Contributing to Richmond

Richmond is the cellular urbanization stack: footprints, rooms, ornamental kit,
and building composition. This document covers how **buildings** should author
and present geometry on top of [`building-components`](building-components/).

## Crates

| Crate | Role |
|-------|------|
| [`building-components`](building-components/) | Domain IR + kit assets. Authoring types are `*Node` values (`FloorNode`, `WallNode`, `StairNode`, `DoorNode`, `RoofNode`): **style + geometry + placement**. Each node implements [`LodScene`](../lod/lib/src/gen/presentation.rs). Tessellation into kit pieces is private to the domain. |
| [`buildings`](buildings/) | Building procedures. Compose constraints, layouts, and helpers (`ArcWall`, `ArcSpire`, …) into **owned collections of nodes** (and rare non-mesh features such as lights). Implement `LodScene` by emitting those nodes (and helpers) under the requested LOD. |
| [`buildings-playground`](buildings-playground/) | Preview / CLI. Prefer constructing a one-off node or a leaf kit type when showing a single piece. Tracks the camera into an [`LodRef`](../lod/lib/src/lod_ref.rs) and re-presents when `scene_lod_status` is `Changed` (whole-scene despawn/spawn for now). |

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
  scene_lod_status(lod_ref)  ──►  short-circuit if Unchanged
  scene_with_lod(lod_ref)    ──►  node.scene_with_lod(lod_ref)
```

Preferred shape for a storey or room:

```rust
pub struct ExampleFloor {
    pub floors: Vec<FloorNode>,
    pub walls: Vec<WallNode>,
    pub stairs: Vec<StairNode>,
    // doors / roofs as they appear
}

impl ExampleFloor {
    pub fn new(/* constraints, noise, … */) -> Self {
        // Layout → construct nodes with Style + Geometry + Placement.
        // Prefer FloorNode::rough_stone(geom, placement), WallNode::rough_stone(…), etc.
        Self { /* … */ }
    }

    fn band_for(&self, viewer: &Transform) -> ExampleLodBand { /* … */ }

    fn is_near(&self, viewer: &Transform) -> bool {
        matches!(self.band_for(viewer), ExampleLodBand::Near)
    }
}
```

Helpers such as `ArcWall` / `ArcSpire` are fine when they **produce** `Vec<WallNode>` / `StairNode`. They should not become a second scene API that bypasses nodes.

Prefer **methods on the building type** (`self.band_for`, `self.emit_external_features`, …) over free module helpers.

## Allocate cells, fill in children

Higher-order room types (e.g. [`Bedroom`](buildings/src/bedroom.rs)) own **layout**: they `subset` child AABBs from [`CellConstraints`](buildings/src/constraints.rs) and construct lower-order types. Lower-order types own **fill**:

| Child | Responsibility |
|-------|----------------|
| `Bed` / `Nightstand` | Place a [`FurnitureNode`](building-components/src/furniture/) scaled to the allocated AABB |
| `Closet` / `EnsuiteBathroom` | Draw partition walls on the shell **and** place furniture/fixture nodes inside |

Do not tessellate furniture or closet walls inside `Bedroom` itself — only allocate and hand constraints down (`Child::new(child_constraints)`).

Constructors take the child's [`CellConstraints`](buildings/src/constraints.rs). Do not pass a parent `&CellConstraints` “for context”; subsetting already baked ownership into the child. Occasional types may also take `&ParentType` when they need authoring detail that constraints cannot express — none of the current bedroom (or tower) children do.

Room layout is **noise-fitted**: [`BedroomLayout::fit`](buildings/src/bedroom/layout.rs) always places **at least one bed**, then greedily adds nightstands / closets / ensuites / further beds. [`BedroomFillParams`](buildings/src/bedroom/layout.rs) controls packing:

- **`spaciousness`** — scales each concept’s base footprint (higher → more floor claimed per item).
- **`occupancy`** — maximum fraction of room floor area to allocate; stop so about `1 - occupancy` stays empty.

Candidates that intersect [`CellConstraints::circulation_exclusion_zones`](buildings/src/constraints/circulation.rs) are rejected (external openings project inward by their along-face width). For **internal** partitions (closet / ensuite), layout reserves a door-swing volume first, check-fits it against beds and other fills, then places the wall body behind that swing so the opening does not project into occupied space.

## `LodScene` on buildings

Every presentable building type still implements `LodScene`:

- `scene_lod_status` — cheap; compare LOD **banding** (or other selection) for `previous_transform` vs `current_transform`. Return `Changed` only when those outcomes differ. Do not build a scene here.
- `scene_with_lod` — scene for the **current** selection only.

```rust
impl LodScene for ExampleFloor {
    fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
        let prev = self.band_for(lod_ref.previous_transform);
        let curr = self.band_for(lod_ref.current_transform);
        if prev == curr {
            LodSceneStatus::Unchanged
        } else {
            LodSceneStatus::Changed
        }
    }

    fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
        let mut children = Vec::new();
        self.emit_external_features(&mut children, lod_ref);
        if self.is_near(lod_ref.current_transform) {
            self.emit_internal_features(&mut children, lod_ref);
        }
        scene_children(children)
    }
}
```

Presenters call `scene_lod_status` first and only build/`handle` when status is `Changed` (or on first present / version repair). Leaves and domain nodes that ignore LOD return `Unchanged`.

### Partition mesh resolution

[`WallNode`](building-components/src/partitions/node.rs) selects rough-stonework **high / mid / low** GLBs from distance ÷ characteristic placement extent ([`PartitionLodBand`](building-components/src/partitions/lod.rs)). UltraLow and Low share the low-res mesh until a shared ultra-low asset exists. `scene_with_lod` always spawns all three [`MeshRef`](../mesh-ref/) children and **hides** inactive tiers so assets stay warm.

Parents should not OR every wall child. Use [`WallNode::representative_lod_status`](building-components/src/partitions/node.rs) at a footprint center with a characteristic extent instead.

### Wizard’s Tower status composition

- Each **floor / perch**: Near/Far flip → `Changed`; otherwise `representative_lod_status` for **that storey** only (ignores internal floors/stairs/walls for status).
- **Column / root**: `Changed` if **any** floor or the perch reports `Changed` (OR of storey statuses). Do not use one tower-wide representative.
- Internals still emit only when Near; their mesh LOD is not composed upward until the storey chooses to emit them.

> [!NOTE]
> Prefer composing storey (or layer) statuses, or a representative partition sample, over walking every leaf. Composites may ignore lower-order scene changes until they are close enough to render those features.

## Internal vs external emission

For LOD, split feature emission into separate methods—commonly **external** (silhouette / shell visible from far away) and **internal** (rooms, stairs, furniture, lanterns that only matter up close).

```rust
impl ExampleFloor {
    fn emit_external_features(
        &self,
        children: &mut Vec<Box<dyn Scene>>,
        lod_ref: &LodRef,
    ) {
        // Outer walls, roof silhouette, façade floors — usually always or at coarser LODs.
        for wall in &self.walls {
            children.push(Box::new(wall.scene_with_lod(lod_ref)));
        }
    }

    fn emit_internal_features(
        &self,
        children: &mut Vec<Box<dyn Scene>>,
        lod_ref: &LodRef,
    ) {
        // Stairs, interior partitions, lights — gate on lod_ref / distance policy.
        for stair in &self.stairs {
            children.push(Box::new(stair.scene_with_lod(lod_ref)));
        }
    }
}
```

Guidance:

- **Name the split after visibility**, not after domain (`emit_external_features` / `emit_internal_features` rather than `emit_walls` alone), so LOD policy stays obvious.
- External features should still be nodes when they are floors/walls/roofs; do not drop back to free `rough_stone_*` scene functions.
- Internal emission may omit whole domains at coarse LOD (e.g. skip stairs and room fills) without deleting them from the authored struct—the IR remains complete; presentation chooses what to show.
- Non-mesh accents (point lights, markers) can ride along in the appropriate emission helper; they are the exception to “nodes only,” not a second geometry pipeline.

Exact LOD thresholds are building- or model-specific; the important contract is that **authoring stays in nodes** and **LOD only filters/composes emission**.

## What not to do

- Do not reintroduce public kit enums or `IntoGeometryComponents` in buildings.
- Do not implement style→asset mapping in buildings; that belongs on `*Node` in building-components.
- Do not treat `Placed<G>` as the public authoring type for new code—prefer `*Node` (use `Placement` when you only need pose).
- Do not leave `LodScene` as a dump of every child with no LOD structure once a building grows past a single LOD band.
- Do not treat “camera moved” as `Changed` by itself — only a change in banding (or other LOD selection) relative to the previous ref.

## Related reading

- [building-components README](building-components/README.md) — Style + Geometry + Placement → `LodScene` pipeline and kit normalization.
- [`LodScene`](../lod/lib/src/gen/presentation.rs) — presentation trait buildings and nodes share.
- [Maybraid contributing: `-models` crates](../CONTRIBUTING.md#-models-crates) — how building *behavior* (physics, generation plugins) layers on top of composition crates like Richmond.
