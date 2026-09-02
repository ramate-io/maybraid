# Firearm intelligence

Higher-order firearm combatant brains.

- [`FirearmSpotting`](src/target.rs) lists candidate enemies. Sightline probes turn visible candidates into remembered [`SpottedTarget`](src/target.rs) snapshots with position, capsule, velocity, and observation time.
- [`FirearmIntelligence`](src/combat.rs) fields `FirearmObjective(Vec<SpottedTarget>)`: accuracy, headshots, focus, trigger happiness, wall firing, and target spotting memory. It writes desired [`PlayerLook`](../../../player/src/identity.rs), then gates the held [`WeaponTrigger`](../../../items/firearms/src/projectiles.rs) from the propagated firearm bore and current obstruction.
- [`FirearmMovementIntelligence`](src/movement.rs) fields `FirearmMovementObjective(Vec<SpottedTarget>)`: range, cover, flee, and vantage. It writes [`MovementObjective`](../../movement/lib/src/objective.rs) and [`ReplanMovement`](../../movement/lib/src/user.rs) from the last observed position.

The spotting-memory duration lives on firearm combat settings and supplies both objective lists. Movement intelligence does not own hide/sightline policy.
