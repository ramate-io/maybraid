# Furniture assemblies

Chico-shaped furniture kits: `BedParams` / `ChairParams` / `ChestParams` / `CounterParams` → `build()` → exploded parts. `unit_from_num` changes [`MaterialRef`](../../material-ref/) palette only.

Parts are posed in the unit slot cube \([-0.5, 0.5]^3\) and composed under a Richmond `FurnitureNode` via `Placement::compose_child` (slot yaw already carries abutment facing). Kit remap (Blender \(Z\)-up → engine \(Y\)-up) is documented in [`src/kit_space.rs`](src/kit_space.rs).

Presentation instances posed cuboids with `MaterialRefRoot` (`RECIPE_WOOD` / `RECIPE_HAY`). Do not `MultiSceneMerge`. When GLBs land at the paths in [`src/assets.rs`](src/assets.rs), swap the cuboid for a `SceneRef` per part.
