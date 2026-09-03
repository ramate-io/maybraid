# Character ragdoll

Persistent procedural corpses for Crozon character visuals.

`CharacterRagdollPlugin` converts a damage [`Downed`](../../damage/src/lifecycle.rs)
body into two lifetimes:

1. The gameplay capsule is retired after its `CharacterRoot` visual is detached.
2. The detached visual becomes a `Corpse`, simulates for a short time, sleeps,
   and expires through `DespawnAfter`.

The solver does not turn rendered bones into physics bodies. It captures named
bones into world-space particles, applies inherited velocity, impact, gravity,
drag, distance constraints, and sphere casts against Fixed geometry, then
marshals that state back through each rig's `BoneMap`. The same solver has
profiles for humanoid, quadruped, and forelimbed body rigs. Forelimbed rigs use
lower gravity and greater drag as a basic buoyancy approximation.

Ragdoll state is keyed by bone name rather than entity, so a refreshed LOD bone
map can receive the current pose. `SuspendAnimation` prevents clip output from
competing with the corpse pose; marshalling runs after structural rig pose and
before transform propagation.

External systems can write `RagdollImpulse` to disturb an awake or sleeping
corpse without depending on solver internals. `CharacterRagdollSettings`
configures corpse lifetime, visual-handoff timeout, and solver iterations.
