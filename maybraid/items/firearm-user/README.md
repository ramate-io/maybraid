# Firearm user

Tracks every world agent that holds a firearm. [`FirearmUser`](src/lib.rs) is a
Bevy relationship onto the kit (`held`); [`HeldBy`](src/lib.rs) is the reverse
index on the gun. Hold / aim knobs live on [`FirearmUserSettings`](src/lib.rs)
(defaults match the firing-range bullpup).

Pose and arm IK key off the user's [`CharacterRoot`](../../crozon/characters/src/member.rs)
child, so an [`Npc`](../../player/src/identity.rs) holds the same way as the
player. Pad fire and the world reticle stay on the followed [`Player`](../../player/src/identity.rs).

Writes player handoff slots ([`PlayerUse`](../../player/src/identity.rs),
[`PlayerCameraAim`](../../player/src/identity.rs),
[`WeaponTrigger`](../firearms/src/projectiles.rs)) and poses the kit + arm IK.
`PlayerLook` follows the camera's −Z convention; the pose preserves its pitch
sign when rotating the firearm's +Z bore around the shoulder-pinned stock.

Does not own the capsule, follow camera, or projectile flight.
