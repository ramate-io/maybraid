# Player camera

Follow camera, POV toggle, look cone, FOV, and a third-person look-stick
zoom lock. Knobs live on
[`FollowCamera`](src/lib.rs) (defaults match the firing-range orbit). Runtime
yaw / pitch / POV / focus live on [`CameraController`](src/look.rs).
[`spawn_follow_camera`](src/lib.rs) also installs a `SpatialListener` so firearm
shots can pan and fall off with distance.

Reads [`PlayerLook`](../player/README.md) slots on the followed body; writes look
back so item users never query `Camera3d`.

[`PlayerCameraAim`](../player/src/identity.rs) is the handoff: a firearm-user writes a
sight pose and the held optic's ADS FOV, a melee-user could write a lock-on pose. Follow
lerps default POV → aim by `focus`. First-person ADS pose uses that blend.
LT / right mouse is optic ADS: FOV uses the aim slot when present, otherwise
[`FollowCamera::sight_fov`](src/lib.rs) (iron sights). Left bumper / middle
mouse / C is the same pose with iron FOV only, so long optics stay usable in
close combat. First-person face hide uses
[`hide_socketed_parts`](../world/layers/mobs/characters/lib/src/member.rs) with
[`CharacterPartSlot::hides_in_first_person`](../world/layers/mobs/characters/lib/src/assembly.rs).

A quick, full-deflection **right-stick flick** (the shared
[`FlickLane`](../input/src/flick.rs) classifier — out, peak, home within 0.32 s)
locks the orbit 10 m from the player in the throw direction. A second flick the
same way steps out to 20 m. The opposite flick steps back in; R3 / look-stick
click releases the lock instead of swapping POV. Ordinary look still pans: the
detector never consumes `look_stick`, and RB (skill-map steer) or LB+RB (Tanzer
toggle chord) reset the zoom lane so those gestures stay exclusive. The world
plugin shapecasts the eased eye so the lock still pulls in of Fixed geometry.
