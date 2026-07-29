# Richmond Building Components

This crate contains various scene components for Richmond buildings.

> [!NOTE]
> All components implement [`lod::gen::LodScene`](../../lod/lib/src/gen/presentation.rs) so they can be used in the scene graph and with generation-presentation flows.

## Layout

Each domain module owns style, geometry, a node IR, and named leaf scene types:

```
floors.rs
floors/geometry.rs      # FloorGeometry
floors/style.rs         # FloorStyle
floors/node.rs          # FloorNode + LodScene
floors/tessellate.rs    # private kit expansion
floors/rough_stonework/
floors/wood/
…
```

Partition (and other) materials use shortened leaf names under the variant folder:

```
partitions/rough_stonework.rs
partitions/rough_stonework/arc_15.rs
partitions/rough_stonework/linear.rs
…
```

Shared [`Placement`](src/placed.rs) / [`Placed`](src/placed.rs) and [`ArcKit`](src/arc_kit.rs) live at the crate root.

## Furniture (placeholders)

[`furniture/`](src/furniture.rs) follows the same Style + Geometry + Placement → `LodScene` IR (`FurnitureNode`). Until kit GLBs exist, [`FurnitureStyle::Placeholder`](src/furniture/style.rs) renders color-coded **wireframe** unit cubes (line-list mesh). Apps must add [`FurnitureWireframePlugin`](src/furniture/wireframe.rs) before spawning furniture scenes. Geometry kinds include bed, wardrobe, nightstand, vanity, and toilet.

## Pipeline

**Style + Geometry + Placement → LodScene**

1. **`*/geometry.rs`** — continuous forms with size and orientation (`Wall::arc(45.0)`, `Floor::rectangle()`, …).
2. **`*/style.rs`** — material / look (`RoughStonework`, `Wood`, …).
3. **`*/node.rs`** — authoring IR (`FloorNode`, `WallNode`, …) that implements `LodScene`: tessellates geometry privately, composes placement, and maps kit pieces to GLBs or leaf placeholders.

Scaling vs repeating continuous forms is deferred; arc decomposition prefers 180° / 90° / 15°.

## Swept Components

We have not yet defined a sweeping tool. The plan is to make it take linear segments in \(X \in [-1.0, 1.0]\) and extrude/fill them along a path (line or arc).

## Partitions

Partition components are authored in a normalized local space, then transformed into world/cell space by the parent building.

- **Linear Normalization:** linear components are normalized to the following spaces:
  - \(Z = [-0.2, 0.2]\)
  - \(Y = [0.0, 1.0]\)
  - \(X = [-1.0, 1.0]\)
  - Subsegments normalized to \(X = [-1.0, 0.8]\)
- **Angular Normalization:** angular components follow a similar normalization along the arc, but attach to different start and end points at different angles.
  - Thickness is the same swept \(Z = [-0.2, 0.2]\)
  - A 180° arc sweep goes through \(-Z\) from \(X = -1.0\) to \(X = 1.0\)
  - A 90° arc sweep goes through \(-Z\) from \(X = -1.0\) to \(X = 0.0\)
  - A 15° arc sweep goes through \(-Z\) from \(X = -1.0\) to \(X = \cos(15^\circ) - 1.0\), \(Z = -\sin(15^\circ)\)
- **Header Components:** header components are used for smaller vertical spaces. They are normalized to:
  - \(Z = [-0.2, 0.2]\)
  - \(Y = [0.0, 0.2]\)
  - \(X = [-1.0, 1.0]\)

A common approach to building door frames is to use a header component with various 15° arc sweeps to create the frame.

## Floors, Roofs, Stairs, and Doors

These modules hold reusable floor/roof fillers, circulation geometry, and door kits. Floors are typically an **arc filler** plus a **struct filler**. Roofs tessellate from a unit right-triangle kit (and empty dome arc kits). Prefer rough stonework for partitions/floors; shepherd's thatch for roofs; wood appears occasionally (interior halfspaces, perch decking, door leaves).

## Floors

Floors components come in three categories:

- **Rectangular:** the floor component is a square centered at the origin with half-length \(1\) (\(X, Z \in [-1, 1]\)) and \(Y = [-0.2, 0.2]\). Often, we square-off more complex forms and fill in the missing space with rectangular components. World edge length \(L\) maps with scale \(L / 2\).
- **Triangular:** the floor component is a unit right triangle with Y = [-0.2, 0.2]. Often, we use triangular components to fill angled sections. 
- **Plank:** the floor component is a rectangle with Z = [-0.2, 0.2], Y = [-1.0, 1.0], and X = [-0.2, 0.2]. Often, we use plank components to fill under complicated polylines, hiding their ends in a partition wall or close to it. They are also quite useful in combination with other rectangular components to fill gaps without aggressive scaling differences per component. 
- **Circle Inscribed Square:** the floor component is the southern- hemisphere difference between a circle and a square. The space removed by the inscribed square is roughly X = Z =[ -0.7, 0.7]. To completely fill in circular space, rotate four of these components around the center.

To fill irregular spaces, we commonly use rectangular or triangular tiling techniques--unless a more bespoke component such as the Circle Inscribed Square is provided. Tiling techniques include quadtree voxelization or a simple sweep of a repeated unit shape. 

## Stairs

Stairs treads are typically authored as X = Y = Z [-1.0, 1.0] unit cubes s.t. the left face of the stairs is in the -Z direction. Often, the author will bleed the geometry will out to X = -2.0, to give support for the stair placed on top. 

When authoring circular stairs, we typically use a fraction of the radius for the tread width and angle each of the treads along the arc.

Tread depths can vary as is needed for connectivity and spacing. 

Tread heights should typically be around 0.18 world units. 

## Roofs

The atomic roof kit is a unit right triangle \(X = Z = [0, 1]\), \(Y = [-0.2, 0.2]\). Continuous forms tessellate into those kits in flat roof-plane space; pitch is then applied as a rotation about **local +X** (run along Z, rise in Y). Parent [`Placement`](src/placed.rs) (scale / yaw / translate) wraps that pitched assembly. Ridges, fascias, and other joinery cover seams.

Public primitives on [`RoofGeometry`](src/roofs/geometry.rs):

- **Rectangular half gable** — tile mirrored right-triangle pairs into unit squares along Z (`length_units`), then pitch.
- **Rectangular intersecting half gable** — same tiling, but the far-end bottom triangle is scaled (`end_triangle_scale`) so a crossing pitch can meet it.
- **Half triangular hip** — a single pitched right triangle.
- **Half trapezoidal hip** — base triangle plus further triangles (`edge_units`) so the roofline is an edge rather than a point.
- **Dome** — continuous sweep decomposed with the same 180° / 90° / 15° [`ArcKit`](src/arc_kit.rs) standard as partitions and floor arc fills. Dome leaves are empty until bespoke GLBs exist.

Buildings-crate roof complexes compose these primitives; kit → GLB mapping stays in this crate (shepherd's thatch right-triangle LOD triad today).

Roof geometry does not fill the entire roof volume. Even domes carve out the inner space. This is intentional, allowing features to sit under the roof surface. Cleverly authored types can delegate these inner spaces to be filled by other components.