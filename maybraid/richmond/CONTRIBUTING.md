# Contributing to Richmond

Richmond is the cellular urbanization stack: footprints, rooms, ornamental kit,
and building composition. This document covers how **buildings** should author
and present geometry on top of [`building-components`](building-components/).

## Crates

| Crate | Role |
|-------|------|
| [`building-components`](building-components/) | Domain IR + kit assets. Authoring types are `*Node` values (`FloorNode`, `WallNode`, `StairNode`, `DoorNode`, `RoofNode`): **style + geometry + placement**. Each node implements [`LodScene`](../lod/lib/src/gen/presentation.rs). Tessellation into kit pieces is private to the domain. |
| [`buildings`](buildings/) | Building procedures. Compose constraints, layouts, and helpers (`ArcWall`, `ArcSpire`, …) into **owned collections of nodes** (and rare non-mesh features such as lights). Implement `LodScene` by emitting those nodes (and helpers) under the requested LOD. |
| [`buildings-playground`](buildings-playground/) | Preview / CLI. Prefer constructing a one-off node or a leaf kit type when showing a single piece. |

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
  LodScene::scene_with_lod  ──►  node.scene_with_lod(lod_ref)
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
}
```

Helpers such as `ArcWall` / `ArcSpire` are fine when they **produce** `Vec<WallNode>` / `StairNode`. They should not become a second scene API that bypasses nodes.

## `LodScene` on buildings

Every presentable building type still implements `LodScene`. The implementation’s job is to **select and compose** already-authored nodes (plus incidental non-node scenes), not to invent kit tessellation.

```rust
impl LodScene for ExampleFloor {
    fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
        let mut children = Vec::new();
        self.emit_external_features(&mut children, lod_ref);
        self.emit_internal_features(&mut children, lod_ref);
        scene_children(children)
    }
}
```

Keep `scene_with_lod` thin: branch on LOD, call emission helpers, group with `scene_children`.

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

## Related reading

- [building-components README](building-components/README.md) — Style + Geometry + Placement → `LodScene` pipeline and kit normalization.
- [`LodScene`](../lod/lib/src/gen/presentation.rs) — presentation trait buildings and nodes share.
- [Maybraid contributing: `-models` crates](../CONTRIBUTING.md#-models-crates) — how building *behavior* (physics, generation plugins) layers on top of composition crates like Richmond.
