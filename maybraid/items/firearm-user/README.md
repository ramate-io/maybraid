# Firearm user

Tracks every world agent that holds a firearm. [`FirearmUser`](src/lib.rs) is a
Bevy relationship onto the kit (`held`); [`HeldBy`](src/lib.rs) is the reverse
index on the gun. Hold / aim knobs live on [`FirearmUserSettings`](src/lib.rs)
(defaults match the firing-range bullpup).

Writes player handoff slots ([`PlayerUse`](../../player/src/identity.rs),
[`PlayerCameraAim`](../../player/src/identity.rs),
[`WeaponTrigger`](../firearms/src/projectiles.rs)) and poses the kit + arm IK.

Does not own the capsule, follow camera, or projectile flight.
