# Furniture

Painted assemblies that fill Richmond [`FurnitureNode`](../richmond/building-components/src/furniture/node.rs) slots. This is a generate/present layer next to Richmond, not a second world binary.

- [`components`](components/) — kit `AssetPath`s, Blender→engine remap, posed `SceneRef` + `MaterialRef`
- [`assemblies`](assemblies/) — `FooParams` → `build()` → exploded parts
- [`playground`](playground/) — isolated `/show` catalog (unit kits + Richmond slot gallery)

Blender sources live in [`art/furniture`](../art/furniture/) ([`ecb5a8a`](https://github.com/ramate-io/maybraid/commit/ecb5a8a31d2894311fa690cfbe8ab01cee385fba)). Export scene 0 into the paths in [`components/src/assets.rs`](components/src/assets.rs) (skip `.blend1`).
