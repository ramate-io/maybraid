# Furniture assemblies

Chico-shaped furniture kits: `BedParams` / `ChairParams` / `ChestParams` / `CounterParams` → `build()` → exploded parts. `unit_from_num` changes [`MaterialRef`](../../material-ref/) palette only.

Parts are floor-origin slabs in the unit slot cube, then [`furniture-components`](../components/) maps each authored GLB through `kit_to_unit` and the Richmond `FurnitureNode` placement (slot yaw already carries abutment facing). Covers use the same transform as the mattress.

[`generation`](src/generation.rs) walks Richmond [`BuildingComponents`](../../richmond/building-components/src/lib.rs) High furniture slots. [`on_buildings`](src/on_buildings.rs) instances painted kits as host children when developments present.

Do not `MultiSceneMerge`. One `SceneRef` per part.
