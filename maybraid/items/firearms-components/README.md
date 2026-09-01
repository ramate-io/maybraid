# `firearms-components`

Domain IR for Maybraid firearms: **kit GLB + socket → node (`LodScene`)**.

Higher-order firearms in [`firearms`](../firearms/) implement [`FirearmComponents`](src/lib.rs) and present via [`ComponentsOnly`](src/lib.rs).

## Domains

| Node | Role |
|------|------|
| [`RigNode`](src/nodes.rs) | Shared receiver armature (`body`, `barrel`, `grip`, `stock`). |
| [`PartNode`](src/nodes.rs) | Body, barrel, grip, stock, or a baked full-concept mesh. |

Until a kit piece exists for a slot, that method returns empty. Armature indexing and pose live in [`rigs`](../../rigs/). The shared receiver is [`firearm_rig.glb`](../../assets/items/guns/firearm_rig.glb).

Blender sources live under [`maybraid/art/items/guns/`](../../art/items/guns/); runtime GLBs under `maybraid/assets/items/guns/`.
