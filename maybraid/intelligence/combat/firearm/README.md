# Firearm intelligence

Higher-order firearm combatant brains.

- [`FirearmSpotting`](src/target.rs) lists candidate enemies. Sightline probes sample the capsule (center, head, hips, and sides) and remember a **visible** aim point. Raycasts ignore an origin already inside Fixed geometry so camped eyes/muzzles are not instantly blind. Observations whose entity left the candidate list are dropped (a respawned player is not a ghost).
- [`FirearmIntelligence`](src/combat.rs) fields `FirearmObjective(Vec<SpottedTarget>)`: accuracy, headshots, focus, trigger happiness, wall firing, spotting memory, and fire freshness. It writes desired [`PlayerLook`](../../../player/src/identity.rs) from the **stock** (the pose pivot) toward the live capsule. Fire still checks the posed muzzle. Alignment is against the capsule, not a remembered sliver. Once the bore is on target, the trigger stays held through a frame of jitter. `trigger_happiness` is only the acquire delay.
- [`FirearmMovementIntelligence`](src/movement.rs) fields `FirearmMovementObjective(Vec<SpottedTarget>)`: range, cover, flee, and vantage. It hunts spotting candidates from their live positions even without a current sightline. If the combat brain does not have a fresh sightline, hide is lowered and sightline weight is raised so the mover leaves a glued cover hole.

Look and hunt may use last-known poses for `target_spotting_memory`. Combat still needs a spotted observation to aim, and a **fresh** one to fire.
