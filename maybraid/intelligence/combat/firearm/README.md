# Firearm intelligence

Higher-order firearm combatant brains.

- [`FirearmIntelligence`](src/combat.rs) fields [`FirearmObjective`](src/target.rs) (`Vec<CombatTarget>`): accuracy, headshots, focus. Writes [`PlayerLook`](../../../player/src/identity.rs) and the held [`WeaponTrigger`](../../../items/firearms/src/projectiles.rs).
- [`FirearmMovementIntelligence`](src/movement.rs) fields [`FirearmMovementObjective`](src/target.rs): range, cover, flee, vantage. Writes [`MovementObjective`](../../movement/lib/src/objective.rs) and [`ReplanMovement`](../../movement/lib/src/user.rs).

Perception (the playground, a sense system) fills both target lists. Movement intelligence does not own hide/sightline policy.
