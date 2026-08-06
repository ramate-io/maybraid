# `chico-vegetation-components`

Domain IR for Chico vegetation: **style + geometry + placement → node (`LodScene`)**.

Higher-order trees implement [`VegetationComponents`](src/lib.rs) and present via [`ComponentsOnly`](src/lib.rs), mirroring Richmond's [`BuildingComponents`](../../richmond/building-components/).

## Domains

| Node | Role |
|------|------|
| [`StickNode`](src/sticks.rs) | Trunk / branch segments. Kit: \(Y \in [0, 1]\), \(X = Z \in [-0.2, 0.2]\). |
| [`FoliageNode`](src/foliage.rs) | Canopy leaves (layered ball, noisy ball, plane splay, …). |

Stick styles `Standard` / `StandardTrunk` load GLB LOD triads from `maybraid/assets/vegetation/sticks/`. Foliage style `Standard` + geometry `LayeredBall` loads `maybraid/assets/vegetation/foliage/standard/layered_ball_001_{high,mid,low}_res.glb`. SDF / inline builders remain as named style variants.

## Apps

Add [`VegetationProceduralPlugin`](src/procedural.rs) before spawning vegetation scenes (registers unit stick/ball meshes and fulfills plane-splay foliage).
