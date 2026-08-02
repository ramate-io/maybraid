# Richmond Buildings

Higher-order building authorship on top of [`richmond-building-components`](../building-components/). This crate emits domain IR (`PartitionNode`, `FloorNode`, `StairNode`, …) via [`BuildingComponents`](../building-components/src/lib.rs); it does **not** tessellate kits or own GLB path strings. Present component-only buildings as [`ComponentsOnly`](../building-components/src/lib.rs)`<T>` unless the type needs a custom `LodScene` (hosts, silhouettes, lights).

## Urban kit model

Art sources live under [`maybraid/art/urban/`](../../art/urban/README.md). Buildings helpers should treat that layout as the kit taxonomy:

**Shared within a style** — geometry reused across many primitive types:

- **`panels/`** — rectangles, right triangles, planar fillers
- **`arcs/`** — 15° / 90° / 180° bodies, slices, frames
- **`joints/`** — joints between abutting partition runs

**Parts** — `parts/` libraries of small reusable pieces (stones, thatch elements, …) used when assembling or detailing kits.

**Domain folders** — `floors/`, `partitions/`, `stairs/`, and similar remain for **function-specific** components and **fast authorship paths**. Prefer these when a kit is tied to one use (floor slab, tread, bespoke partition) rather than promoting everything into the shared layer.

Runtime paths are registered in building-components [`assets.rs`](../building-components/src/assets.rs). Shared kits are often **aliased** into domain consumers (e.g. partition `LINEAR_*` → panel rectangle; `ARC_*` / `SLICE_*` → `arcs/`; `JOINT_*` → `joints/`).

## What this crate owns

| Module | Role |
|--------|------|
| [`arcs`](src/arcs.rs) | Fitted circular [`ArcSweep`](src/arcs/sweep.rs) / [`ClippedArcSweep`](src/arcs/clipped_sweep.rs) / [`portal_ring`](src/arcs/portal_ring.rs) (not IR `partitions::ArcSweep`); ellipses deferred as a sibling |
| [`paneling`](src/paneling.rs) | Irregular panel primitives → [`PanelComplex`](src/paneling/panel_complex.rs) / rectangle kits + crease joints |
| [`portals`](src/portals.rs) | Portal vocabulary + assignment along unit path \(t\) |
| [`wall_demo`](src/wall_demo.rs) | Playground joinery demos (e.g. noisy path → rectangle strip) |
| [`wizards_tower`](src/wizards_tower.rs) | Authored tower hierarchy |
| [`bedroom`](src/bedroom.rs) | Hierarchical room fill |
| [`stacked_rings`](src/stacked_rings.rs) | Circular wall stack |
| [`arc_spire`](src/arc_spire.rs) | Spire / storey binding helpers |
| [`constraints`](src/constraints.rs) | Cell / boundary / circulation IR |
| [`openings`](src/openings.rs) | Opening plans / shell records / mapped contact geometry |
| [`shells`](src/shells.rs) | Envelope shells (`ArcFloor`, `Trazaloid`, `ConnectingHall`, …) |

### `paneling` contents

| Type | Role |
|------|------|
| [`PanelComplex`](src/paneling/panel_complex.rs) | Point-id triangle/quad mesh → panels + crease `JointNode`s |
| [`QuadPanel`](src/paneling/quad_panel.rs) / [`QuadPanelComplex`](src/paneling/quad_panel_complex.rs) | Ruled quad / quad-face mesh wrappers |
| [`RuledStrip`](src/paneling/ruled_strip.rs) / [`RuledPitch`](src/paneling/ruled_pitch.rs) | Two-rail skew quads; roof eave/ridge wrapper |
| [`RectangularStrip`](src/paneling/rectangular_strip.rs) / [`ClippedRectangularStrip`](src/paneling/clipped_rectangular_strip.rs) | Two-rail best-fit `PanelGeometry::Rectangle` kits (+ optional per-bay inset frames) + crease joints on bay folds |
| [`Rectangle`](src/paneling/rectangle.rs) / [`ClippedRectangle`](src/paneling/rectangle.rs) | Single-bay best-fit rectangle kit; inset = frame of rectangle kits (not polygonal clip) |
| [`ClippedTessellatedTriangle`](src/paneling/clipped_tessellated_triangle.rs) / [`ClippedQuadPanel`](src/paneling/clipped_quad_panel.rs) / [`ClippedRuledStrip`](src/paneling/clipped_ruled_strip.rs) | Clipped triangle / quad / ruled strip |
| [`TessellatedTrianglePanel`](src/paneling/tessellated_triangle_panel.rs) | World-space triangle → panels |
| [`panel_plane`](src/paneling/panel_plane.rs) | Shared panel \(XZ\) frame for a world triangle |

Crate-root re-exports keep older `richmond_buildings::RuledStrip` paths working.

Authoring guidance for Richmond (LOD, `ParentConfines`, wall vs partition) lives in [`../CONTRIBUTING.md`](../CONTRIBUTING.md). Kit normalization and leaf styles live in the [building-components README](../building-components/README.md).
