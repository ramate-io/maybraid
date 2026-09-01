# `firearms-components`

Domain IR for Maybraid firearms: **kit GLB + socket → node (`LodScene`)**.

Higher-order firearms in [`firearms`](../firearms/) implement [`FirearmComponents`](src/lib.rs) and present via [`ComponentsOnly`](src/lib.rs).

## Domains

| Node | Role |
|------|------|
| [`RigNode`](src/nodes.rs) | Optional receiver armature. Kit parts socket onto named bones (`barrel`, `grip`). |
| [`PartNode`](src/nodes.rs) | Body, barrel, grip, or a baked full-concept mesh. |

Until a receiver rig GLB exists, socket fulfill parents kit parts under the firearm root at authored pose. The same [`SocketRef`](src/socket.rs) will parent under bones once a [`RigNode`](src/nodes.rs) is present.

Blender sources live under [`maybraid/art/items/guns/`](../../art/items/guns/); runtime GLBs under `maybraid/assets/items/guns/`.
