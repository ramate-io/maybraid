# `firearms-components`

Domain IR for Maybraid firearms: **kit GLB + socket → node (`LodScene`)**.

Higher-order firearms in [`firearms`](../firearms/) implement [`FirearmComponents`](src/lib.rs) and present via [`ComponentsOnly`](src/lib.rs).

Art and armature conventions live in [`maybraid/art/items/guns/README.md`](../../art/items/guns/README.md).

## Kit slots

| Node / trait method | Socket bone | Art folder |
|---------------------|-------------|------------|
| [`RigNode`](src/nodes.rs) | — | [`guns/rigs/`](../../art/items/guns/rigs/) |
| `body_nodes_for_level` | `body` | [`guns/bodies/`](../../art/items/guns/bodies/) |
| `barrel_nodes_for_level` | `barrel` | [`guns/barrels/`](../../art/items/guns/barrels/) |
| `trigger_box_nodes_for_level` | `trigger_box` | [`guns/trigger_boxes/`](../../art/items/guns/trigger_boxes/) |
| `grip_nodes_for_level` | `grip` | [`guns/grips/`](../../art/items/guns/grips/) |
| `stock_nodes_for_level` | `stock` | [`guns/stocks/`](../../art/items/guns/stocks/) |
| [`PartNode`](src/nodes.rs) `Concept` | unsocketed | [`guns/concepts/`](../../art/items/guns/concepts/) |

Until a kit piece exists for a slot, that method returns empty. **Body is required** at the recipe layer ([`FirearmKit`](../firearms/src/kit.rs)); barrel, trigger box, grip, and stock may be `none`. Trigger boxes used to be joined with bodies; they are a separate slot so the hull and fire-control box swap independently.

Armature indexing and pose live in [`rigs`](../../rigs/). The shared receiver is [`firearm_rig.glb`](../../assets/items/guns/rigs/firearm_rig.glb).

## Bone space, not `AlongBone`

[`BoneScale::length`](../../rigs/src/pose.rs) scales **local Y**. Kit GLBs are authored so that axis is the part's length; [`SocketRef`](src/nodes.rs) stays identity. Do not add an `AlongBone` remap, and do not hang length-extending parts on extra perpendicular `_socket` bones (those are for attachments such as `grip_point` / `trigger_point`).

Hands bind to `grip_point` and `trigger_point` on the receiver, not to the grip or trigger-box meshes. See the [guns README](../../art/items/guns/README.md) for the rest-pose tree and lengthening rules (`body` still parents `barrel` / `grip`).

## Assets

Blender sources live under [`maybraid/art/items/guns/`](../../art/items/guns/); runtime GLBs mirror that layout under `maybraid/assets/items/guns/`.
