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

Shared [`Placement`](src/placed.rs) / [`Placed`](src/placed.rs), [`ArcKit`](src/arc_kit.rs), and panel tessellation ([`panels`](src/panels/)) live at the crate root.

## Shared panels (`panels/`)

Reusable linear-panel geometry shared by floors and partitions (roofs use roof-native [`Pitch`](src/roofs/geometry.rs)):

| Type | Role |
|------|------|
| [`PanelGeometry`](src/panels/geometry.rs) | Shared enum: rectangle / right-triangle / tessellated-triangle / joint |
| [`PanelStyle`](src/panels/geometry.rs) | Kit capabilities (`has_rectangle`); domain styles map into it |
| [`Rectangle`](src/panels/geometry.rs) / [`RightTriangle`](src/panels/geometry.rs) | Atomic kit footprints (lower-left panel space) |
| [`TessellatedTriangle`](src/panels/tessellated_triangle.rs) | Three world points filled with posed right-triangle kits |
| [`Joint`](src/panels/joint.rs) | Corner filler on the average inbound/outbound angle |

**Panel space:** lower-left anchored — **X** along length, **Z** depth/run (top/eave at \(Z = 0\), bottom/ridge at \(Z = -\texttt{depth}\)). Domain nodes own extra orientation (roof pitch about \(+X\), wall upright framing, floor flat).

Decomposition is style-agnostic geometry (no `LodScene` required); style only chooses rectangle vs dual-triangle body fill:

```text
TessellatedTriangle.decompose() → Placed<RightTriangle>
PanelGeometry::flatten(style)   → Placed<Rectangle | RightTriangle | Joint>
```

[`PanelStyle::has_rectangle`](src/panels/geometry.rs) is a kit capability (e.g. shepherd's thatch → triangles only; rough stonework → rectangles). Domains map `DomainStyle → PanelStyle`, flatten geometry, then map atoms to kits/GLBs. Shared placement remaps live in [`kit_space`](src/panels/kit_space.rs). Plan/slope helpers (`yaw_along_xz`, `roll_along_slope`) live in [`placement`](src/panels/placement.rs).

[`FloorGeometry`](src/floors/geometry.rs) and [`PartitionGeometry`](src/partitions/geometry.rs) expose `TessellatedTriangle`. Higher-order polyline-of-lines paneling lives in [`richmond_buildings::DividedPaneling`](../buildings/src/divided_paneling.rs).

## Urban art / assets

Blender sources live under [`maybraid/art/urban/`](../../art/urban/README.md); runtime GLBs under `maybraid/assets/urban/` (same relative layout). Paths are registered in [`assets.rs`](src/assets.rs).

Urban kits split into **shared style geometry**, **parts**, and **domain-specific** folders:

- **`panels/`**, **`arcs/`**, **`joints/`** — widely reusable geometry within a style (rectangles/triangles, arc bodies/slices/frames, joints), consumed across partitions, roofs, floors, doors, …
- **`parts/`** — reusable micro-pieces for a style (stones, thatch tufts, …); authoring libraries more often than direct runtime leaves
- **`floors/`**, **`partitions/`**, **`stairs/`**, … — function-specific kits and **fast authorship paths** when a component stays tied to one domain

```
urban/
  panels/                 # shared rectangles, triangles, fillers
    unit_right_triangle
    rough_stonework/
      rectangle_001[+ LOD]
      inscribed_square_001
    shepherds_thatch/
      right_triangle_001_{high,mid,low}_res
  arcs/                   # shared arc bodies, slices, frames
    rough_stonework/
      arc_{15,90,180}_001[+ LOD]
      arc_{15,90}_slice_001[+ LOD]
      arc_90_frame_001
  joints/                 # shared segment joints
    rough_stonework/
      joint_001_{high,mid}_res
  parts/                  # style piece libraries (stones, …)
    rough_stonework/
      parts
    shepherds_thatch/
      parts
  floors/
    rough_stonework/
      rough_stonework_001   # floor-slab rectangle (domain-owned for now)
  stairs/
    …
```

**Naming under a style folder:** do not repeat the style name in the filename (`rough_stonework/arc_90_001`, not `rough_stonework/rough_stonework_90_001`). Angle kits use the `arc_` prefix (`arc_15`, `arc_90`, `arc_180`, plus `arc_*_slice` / `arc_*_frame`).

Partition linear leaves consume `panels/.../rectangle_001` via `LINEAR_*` aliases; roof pitch leaves consume `panels/.../right_triangle_001_*`. Arc / slice / joint leaves alias through `ARC_*` / `SLICE_*` / `JOINT_*` into `arcs/` and `joints/`. The floor-slab rectangle remains under `floors/` until promoted; the circle−inscribed-square filler lives under `panels/`.

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

[`Partition::polyline`](src/partitions/geometry/polyline.rs) is a **short-run** thin-wall primitive: one [`PartitionNode`](src/partitions/node.rs) is a single LOD parent whose `scene_with_level` expands into posed linear + joint kits. Prefer splitting longer paths in higher-order constructs (`richmond_buildings::walling`). For panel fills over a polyline of dividing lines, use [`richmond_buildings::DividedPaneling`](../buildings/src/divided_paneling.rs).

Each edge uses **horizontal** length \(L_{xz}\) with a suggested [`tile_width`](src/partitions/geometry/linear.rs) (default \(1\), unscaled ground kit \(X \in [0, 1]\)): \(n = \mathrm{round}(L_{xz}/\texttt{tile\_width})\) tiles stretch to width \(L_{xz}/n\). Starts lerp along the 3D path so path \(Y\) carries slope. Override with `with_tile_width`. Continuous [`LinearPartition::spanning`](src/partitions/geometry/linear.rs) uses the same fit on its span. Polyline tiles carry world path anchors plus stand-up pitch and wall scale themselves (`with_wall_scale`); the parent stays identity. Panels stay **plumb** (yaw + stand-up only).

Joints omit when both plan and slope kinks are below [`DEFAULT_MIN_JOINT_ANGLE`](src/panels/placement.rs) (override via `with_min_joint_angle`). Use `with_incoming_slope` when a split span continues a preceding segment that is not in `points`. Horizontal joint scale grows with the vertical kink; joints and panels stay plumb. Joint meshes follow the **parent** level (high/mid only). LOD policy lives beside each geometry variant under [`geometry/`](src/partitions/geometry/).

## Partitions

Partition components are authored in a normalized local space, then transformed into world/cell space by the parent building.

- **Linear / rectangle panel:** authored on the ground like the triangle panel:
  - \(X, Z \in [0, 1]\)
  - \(Y \in [-0.2, 0.2]\) (thickness)
  - Wall use scales \((\texttt{length}, \texttt{thick}, \texttt{height})\) on \((X, Y, Z)\), then pitches \(\pi/2\) about \(+X\) so kit \(+Z\) stands up as height (kit \(Y\) becomes thickness). Panels stay **plumb** (yaw + stand-up only); polyline slope lives in path \(Y\), not tip roll. Segments anchor at the lower-left (kit origin). Kit \(X \in [0, 1]\) means length scale is the **full** span (half of the old \(X \in [-1, 1]\) half-extent convention). See [`wall_placement`](src/partitions/geometry/linear.rs).
- **Angular Normalization:** angular components (`arc_180` / `arc_90` / `arc_15`) follow a similar normalization along the arc, but attach to different start and end points at different angles.
  - Thickness is the same swept \(Z = [-0.2, 0.2]\)
  - A 180° arc sweep goes through \(-Z\) from \(X = -1.0\) to \(X = 1.0\)
  - A 90° arc sweep goes through \(-Z\) from \(X = -1.0\) to \(X = 0.0\)
  - A 15° arc sweep goes through \(-Z\) from \(X = -1.0\) to \(X = \cos(15^\circ) - 1.0\), \(Z = -\sin(15^\circ)\)
- **Slice Components:** slice components (`arc_90_slice` / `arc_15_slice`) are used for smaller vertical spaces. They are normalized to:
  - \(Z = [-0.2, 0.2]\)
  - \(Y = [0.0, 0.2]\)
  - \(X = [-1.0, 1.0]\)

A common approach to building door frames is to use a slice component with various 15° arc sweeps to create the frame.

Joints are used to connect irregular partition geometry. They remain **vertically authored** (\(X = Z = [-0.5, 0.5]\), \(Y = [0.0, 1.0]\)) — do **not** apply rectangle-panel stand-up pitch, and do **not** tip them with slope roll yet (stay plumb). Scale \(Y\) so the joint spans the storey; grow \(X/Z\) with the vertical angle kink between abutting segments. Yaw bisects the plan turn. Omit joints when kinks are below the construction `min_joint_angle` (default \(0.1\) rad). When a polyline is split, pass `incoming_slope` so the start vertex can still joint against the preceding segment.

## Floors, Roofs, Stairs, and Doors

These modules hold reusable floor/roof fillers, circulation geometry, and door kits. Floors are typically an **arc filler** plus a **struct filler**. Roofs use roof-native [`Pitch`](src/roofs/geometry.rs) (rectangle + optional end triangles) or **Dome**. Prefer rough stonework for partitions/floors; shepherd's thatch for roofs; wood appears occasionally (interior halfspaces, perch decking, door leaves).

## Floors

Floors components come in three categories:

- **Rectangular:** the floor component is a square centered at the origin with half-length \(1\) (\(X, Z \in [-1, 1]\)) and \(Y = [-0.2, 0.2]\). Often, we square-off more complex forms and fill in the missing space with rectangular components. World edge length \(L\) maps with scale \(L / 2\). Kit: `floors/rough_stonework/rough_stonework_001`.
- **Triangular:** the floor component is a unit right triangle with Y = [-0.2, 0.2]. Often, we use triangular components to fill angled sections. [`TessellatedTriangle`](src/panels/tessellated_triangle.rs) fills an arbitrary 3D triangle with these kits.
- **Plank:** the floor component is a rectangle with Z = [-0.2, 0.2], Y = [-1.0, 1.0], and X = [-0.2, 0.2]. Often, we use plank components to fill under complicated polylines, hiding their ends in a partition wall or close to it. They are also quite useful in combination with other rectangular components to fill gaps without aggressive scaling differences per component. 
- **Circle Inscribed Square:** the floor component is the southern- hemisphere difference between a circle and a square. The space removed by the inscribed square is roughly X = Z =[ -0.7, 0.7]. To completely fill in circular space, rotate four of these components around the center. Kit: `panels/rough_stonework/inscribed_square_001`.

To fill irregular spaces, we commonly use rectangular or triangular tiling techniques--unless a more bespoke component such as the Circle Inscribed Square is provided. [`TessellatedTriangle`](src/panels/tessellated_triangle.rs) and higher-order [`DividedPaneling`](../buildings/src/divided_paneling.rs) are the preferred authoring path for moderate tiled regions; tiling techniques also include quadtree voxelization or a simple sweep of a repeated unit shape.

## Stairs

Stairs treads are typically authored as X = Y = Z [-1.0, 1.0] unit cubes s.t. the left face of the stairs is in the -Z direction. Often, the author will bleed the geometry will out to X = -2.0, to give support for the stair placed on top. 

When authoring circular stairs, we typically use a fraction of the radius for the tread width and angle each of the treads along the arc.

Tread depths can vary as is needed for connectivity and spacing. 

Tread heights should typically be around 0.18 world units. 

## Roofs

Roof IR is [`Pitch`](src/roofs/geometry.rs) or [`Dome`](src/roofs/geometry.rs).

### Pitch

A pitched face is a **rectangle** (optional) plus optional **end triangles**, with parallel eave and ridge on the rectangular body. Trapezoid asymmetry comes only from the ends. Tessellation is **roof-native** (direct `Rectangle` / `RightTriangle` leaves — not shared panel composites). Shepherd's thatch maps to [`PanelStyle::TRIANGLES_ONLY`](src/panels/geometry.rs) so the body fills with dual right triangles.

Pitch-space axes: **X** along eave/ridge, **Z** run (eave at \(Z = 0\), ridge at \(Z = -\texttt{run}\)), **Y** rise via rotation about +X by \(\operatorname{atan2}(\texttt{rise}, \texttt{run})\). Anchor is the **lower-left** of the full extent (left end triangle if present, else the rectangle). `rise` / `run` are non-negative; flip the face with placement rotation instead of negative rise/run.

| Field | Role |
|-------|------|
| `rise` / `run` | Slope; kit Z scaled by `run` |
| `length` | `Option` — rectangular span along X; omit for ends-only |
| `tile_width` | Suggested tile width; \(n = \mathrm{round}(\texttt{length}/\texttt{tile\_width})\) tiles stretch to fit `length` |
| `left` / `right` | `Option` absolute end-triangle base lengths; **positive** = upright (eave-long), **negative** = flipped (ridge-long) |

Helpers: `with_left` / `with_right`, `with_left_angle` / `with_right_angle` (\(\texttt{base} = \texttt{run}\tan\theta\)), and `from_eave_ridge(rise, run, eave, ridge, tile_width)` which sets `length = min(eave, ridge)` and equal end bases \(\pm|\texttt{ridge}-\texttt{eave}|/2\) (flipped when ridge is longer).

The atomic kit is the origin-anchored unit right triangle \(X \in [0, 1]\), \(Z \in [-1, 0]\), \(Y \in [-0.2, 0.2]\) — shepherd's thatch LOD triad under `panels/shepherds_thatch/right_triangle_001_*` (plus style-agnostic `panels/unit_right_triangle`).

### Dome

Continuous sweep decomposed with the same 180° / 90° / 15° [`ArcKit`](src/arc_kit.rs) standard as partitions and floor arc fills. Dome leaves are empty until bespoke GLBs exist.

Roof geometry does not fill the entire roof volume. Even domes carve out the inner space. This is intentional, allowing features to sit under the roof surface. Cleverly authored types can delegate these inner spaces to be filled by other components.
