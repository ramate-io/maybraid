# Furniture assemblies

Chico-shaped furniture kits: `BedParams` / `ChairParams` / `ChestParams` / `CounterParams` → `build()` → exploded parts. `unit_from_num` changes [`MaterialRef`](../../material-ref/) palette only.

Parts are floor-origin slabs in the unit slot cube, then [`furniture-components`](../components/) maps each authored GLB through `kit_to_unit` and the Richmond `FurnitureNode` placement (slot yaw already carries abutment facing). Covers use the same transform as the mattress.

[`generation`](src/generation.rs) walks Richmond [`BuildingComponents`](../../richmond/building-components/src/lib.rs) High furniture slots. [`stream`](src/stream.rs) bins those slots into 50 m [`FurnitureCell`](src/host.rs) hosts and presents a neighborhood around the camera (one cell generate / present / despawn per frame). Each host's `scene_chunks` is one lazy slot that expands to one weighted chunk per painted GLB. Do not parent kits under Richmond development hosts.

Do not `MultiSceneMerge`. One `SceneRef` per part.
