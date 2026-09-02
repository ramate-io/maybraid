# Player camera

Follow camera, POV toggle, look cone, and FOV. Knobs live on
[`FollowCamera`](src/lib.rs) (defaults match the firing-range orbit). Runtime
yaw / pitch / POV / focus live on [`CameraController`](src/look.rs).

Reads [`PlayerLook`](../player/README.md) slots on the followed body; writes look
back so item users never query `Camera3d`.

[`PlayerCameraAim`](../player/src/identity.rs) is the handoff: a firearm-user writes a
sight pose, a melee-user could write a lock-on pose. Follow lerps default POV → aim by
`focus`. First-person face hide uses
[`hide_socketed_parts`](../crozon/characters/src/member.rs) with
[`CharacterPartSlot::hides_in_first_person`](../crozon/characters/src/assembly.rs).
