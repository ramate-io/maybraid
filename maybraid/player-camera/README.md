# Player camera

Follow camera, POV toggle, look cone, and FOV. Reads [`PlayerLook`](../player/README.md)
slots on the followed body; writes look back so item users never query `Camera3d`.

[`PlayerCameraAim`](../player/src/identity.rs) is the handoff: a firearm-user writes a
sight pose, a melee-user could write a lock-on pose. Follow lerps default POV → aim by
`focus`.
