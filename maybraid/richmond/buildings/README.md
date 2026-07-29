# Richmond Buildings

Higher-order building authorship on top of [`richmond-building-components`](../building-components/). This crate emits domain IR (`PartitionNode`, `FloorNode`, `StairNode`, …); it does **not** tessellate kits or own GLB path strings.

## Urban kit model

Art sources live under [`maybraid/art/urban/`](../../art/urban/README.md). Buildings and walling helpers should treat that layout as the kit taxonomy:

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
| [`walling`](src/walling.rs) | Portal-sensitive walls (`LinearWall`, `PolylineWall`, `ArcWall`, …) → `PartitionNode`s |
| [`wizards_tower`](src/wizards_tower.rs) | Authored tower hierarchy |
| [`bedroom`](src/bedroom.rs) | Hierarchical room fill |
| [`stacked_rings`](src/stacked_rings.rs) | Circular wall stack |
| [`arc_spire`](src/arc_spire.rs) | Spire / storey binding helpers |
| [`constraints`](src/constraints.rs) | Cell / boundary / circulation IR |

Authoring guidance for Richmond (LOD, `ParentConfines`, wall vs partition) lives in [`../CONTRIBUTING.md`](../CONTRIBUTING.md). Kit normalization and leaf styles live in the [building-components README](../building-components/README.md).
