# Firearm user

Tracks every world agent that holds a firearm. Writes player handoff slots
([`PlayerUse`](../../player/src/identity.rs), [`PlayerCameraAim`](../../player/src/identity.rs),
[`WeaponTrigger`](../firearms/src/projectiles.rs)) and poses the kit + arm IK.

Does not own the capsule, follow camera, or projectile flight.
