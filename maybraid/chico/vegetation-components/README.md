# `chico-vegetation-components`

Domain IR for Chico vegetation: **style + geometry + placement → node (`LodScene`)**.

Higher-order trees implement [`VegetationComponents`](src/lib.rs) and present via [`ComponentsOnly`](src/lib.rs), mirroring Richmond's [`BuildingComponents`](../../richmond/building-components/).

## Domains

| Node | Role |
|------|------|
| [`StickNode`](src/sticks/) | Trunk / branch segments. Kit: \(Y \in [0, 1]\), \(X = Z \in [-0.2, 0.2]\). |
| [`FoliageNode`](src/foliage/) | Canopy leaves (noisy ball, plane splay, …). |

SDF / inline builders remain as **style variants** until GLBs under `maybraid/art/vegetation/` replace them.

## Apps

Add [`VegetationProceduralPlugin`](src/procedural.rs) before spawning vegetation scenes (registers unit stick/ball meshes and fulfills plane-splay foliage).
