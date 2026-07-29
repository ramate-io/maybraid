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

1. **`*/geometry.rs`** — continuous forms with size and orientation (`Partition::arc(45.0)`, `Floor::rectangle()`, …).
2. **`*/style.rs`** — material / look (`RoughStonework`, `Wood`, …).
3. **`*/node.rs`** — authoring IR (`FloorNode`, `PartitionNode`, …) that implements `LodScene`: tessellates geometry privately, composes placement, and maps kit pieces to GLBs or leaf placeholders.

Scaling vs repeating continuous forms is deferred; arc decomposition prefers 180° / 90° / 15°.

## Swept Components

We have not yet defined a sweeping tool. The plan is to make it take linear segments in \(X \in [-1.0, 1.0]\) and extrude/fill them along a path (line or arc).

## Polyline partitions

[`Partition::polyline`](src/partitions/geometry/polyline.rs) is a **short-run** primitive: one [`PartitionNode`](src/partitions/node.rs) is a single LOD parent whose `scene_with_level` expands into posed linear + joint kits. Prefer splitting longer paths in higher-order constructs (`richmond_buildings::walling`).

Each edge of length \(L\) uses a suggested [`tile_width`](src/partitions/geometry/linear.rs) (default \(2\), the unscaled kit full width): \(n = \mathrm{round}(L/\texttt{tile\_width})\) tiles stretch to width \(L/n\). Override with `with_tile_width`. Continuous [`LinearPartition::spanning`](src/partitions/geometry/linear.rs) uses the same fit.

Joints omit when both plan and slope kinks are below [`DEFAULT_MIN_JOINT_ANGLE`](src/partitions/geometry/polyline.rs) (override via `with_min_joint_angle`). Use `with_incoming_slope` when a split span continues a preceding segment that is not in `points`. Horizontal joint scale grows with the vertical kink; roll averages abutting slopes. Joint meshes follow the **parent** level (high/mid only). LOD policy lives beside each geometry variant under [`geometry/`](src/partitions/geometry/).

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

Joints are used to connect irregular partition geometry. They are roughly circular components defined \(X = Z = [-0.5, 0.5]\) and \(Y = [0.0, 1.0]\). Scale \(Y\) so the joint spans the storey; grow \(X/Z\) with the vertical angle kink between abutting segments. Align roll to the average of those segments' slopes (yaw bisects the plan turn). Omit joints when kinks are below the construction `min_joint_angle` (default \(0.1\) rad). When a polyline is split, pass `incoming_slope` so the start vertex can still joint against the preceding segment.

## Floors, Roofs, Stairs, and Doors

These modules hold reusable floor/roof fillers, circulation geometry, and door kits. Floors are typically an **arc filler** plus a **struct filler**. Roofs use a unified **Pitch** (rectangle + optional end triangles) or **Dome**. Prefer rough stonework for partitions/floors; shepherd's thatch for roofs; wood appears occasionally (interior halfspaces, perch decking, door leaves).

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

Roof IR is [`Pitch`](src/roofs/geometry.rs) or [`Dome`](src/roofs/geometry.rs).

### Pitch

A pitched face is a **rectangle** (optional) plus optional **end triangles**, with parallel eave and ridge on the rectangular body. Trapezoid asymmetry comes only from the ends.

Pitch-space axes: **X** along eave/ridge, **Z** run (eave at \(Z = 0\), ridge at \(Z = -\texttt{run}\)), **Y** rise via rotation about +X by \(\operatorname{atan2}(\texttt{rise}, \texttt{run})\). Anchor is the **lower-left** of the full extent (left end triangle if present, else the rectangle). `rise` / `run` are non-negative; flip the face with placement rotation instead of negative rise/run.

| Field | Role |
|-------|------|
| `rise` / `run` | Slope; kit Z scaled by `run` |
| `length` | `Option` — rectangular span along X; omit for ends-only |
| `tile_width` | Suggested tile width; \(n = \mathrm{round}(\texttt{length}/\texttt{tile\_width})\) tiles stretch to fit `length` |
| `left` / `right` | `Option` absolute end-triangle base lengths; **positive** = upright (eave-long), **negative** = flipped (ridge-long) |

Helpers: `with_left` / `with_right`, `with_left_angle` / `with_right_angle` (\(\texttt{base} = \texttt{run}\tan\theta\)), and `from_eave_ridge(rise, run, eave, ridge, tile_width)` which sets `length = min(eave, ridge)` and equal end bases \(\pm|\texttt{ridge}-\texttt{eave}|/2\) (flipped when ridge is longer).

The atomic kit is still the origin-anchored unit right triangle \(X \in [0, 1]\), \(Z \in [-1, 0]\), \(Y \in [-0.2, 0.2]\) (shepherd's thatch LOD triad).

### Dome

Continuous sweep decomposed with the same 180° / 90° / 15° [`ArcKit`](src/arc_kit.rs) standard as partitions and floor arc fills. Dome leaves are empty until bespoke GLBs exist.

Roof geometry does not fill the entire roof volume. Even domes carve out the inner space. This is intentional, allowing features to sit under the roof surface. Cleverly authored types can delegate these inner spaces to be filled by other components.