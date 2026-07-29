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

[`Partition::polyline`](src/partitions/geometry.rs) takes 3D points and expands (privately) into upright linear kits plus **joints** at plan-angle and elevation kinks. Joints are omitted when both kink angles are below [`DEFAULT_MIN_JOINT_ANGLE`](src/partitions/geometry.rs) (override via [`PolylinePartition::with_min_joint_angle`](src/partitions/geometry.rs)). Horizontal joint scale grows with the vertical (slope) kink; roll is the average of the abutting segment slopes (yaw bisects the plan turn). \(Y\) scale follows the parent storey height. Rough-stone joints ship high + mid GLBs only — low / ultra-low LOD hide them. Portal-sensitive path walls live in `richmond_buildings::walling` (`PolylineWall` / `Walling`), not in this crate.

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

Joints are used to connect irregular partition geometry. They are roughly circular components defined \(X = Z = [-0.5, 0.5]\) and \(Y = [0.0, 1.0]\). Scale \(Y\) so the joint spans the storey; grow \(X/Z\) with the vertical angle kink between abutting segments. Align roll to the average of those segments' slopes (yaw bisects the plan turn). Omit joints when kinks are below the construction `min_joint_angle` (default \(0.1\) rad).

## Floors, Roofs, Stairs, and Doors

These modules hold reusable floor/roof fillers, circulation geometry, and door kits. Floors and roofs are typically an **arc filler** plus a **struct filler**. Prefer rough stonework; wood appears occasionally (interior halfspaces, perch decking, door leaves).

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
