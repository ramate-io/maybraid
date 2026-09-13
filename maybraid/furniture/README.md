# Furniture

Painted assemblies that fill Richmond [`FurnitureNode`](../richmond/building-components/src/furniture/node.rs) slots. This is a generate/present layer next to Richmond, not a second world binary.

- [`assemblies`](assemblies/) — `FooParams` → `build()` → exploded parts with `MaterialRef` paint
- [`playground`](playground/) — isolated `/show` catalog (unit kits + Richmond slot gallery)

Blender sources live in [`art/furniture`](../art/furniture/). Export scene 0 into the `AssetPath`s in [`assemblies/src/assets.rs`](assemblies/src/assets.rs) (skip `.blend1`). Until those GLBs land, assemblies instance procedural cuboids that occupy the remapped kit AABB.
