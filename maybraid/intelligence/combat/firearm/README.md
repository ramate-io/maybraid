# Firearm intelligence

Higher-order firearm combatant brains.

- [`FirearmSpotting`](src/target.rs) lists candidate enemies. Sightline probes sample the capsule (center, head, hips, and sides) and remember a **visible** aim point. An origin skip keeps observers from going blind when their eyes clip into nearby Fixed geometry.
- [`FirearmIntelligence`](src/combat.rs) fields `FirearmObjective(Vec<SpottedTarget>)`: accuracy, headshots, focus, trigger happiness, wall firing, spotting memory, and fire freshness. It writes desired [`PlayerLook`](../../../player/src/identity.rs) from the posed muzzle toward the last remembered visible point. Fire requires that observation to be fresh (~0.2s). Once the bore is on that point, it holds [`WeaponTrigger`](../../../items/firearms/src/projectiles.rs) and lets the weapon interval be the rate of fire. `trigger_happiness` is only the acquire delay.
- [`FirearmMovementIntelligence`](src/movement.rs) fields `FirearmMovementObjective(Vec<SpottedTarget>)`: range, cover, flee, and vantage. It hunts spotting candidates from their live positions even without a current sightline. If the combat brain does not have a fresh sightline, hide is lowered and sightline weight is raised so the mover leaves a glued cover hole.

Look and hunt may use last-known poses for `target_spotting_memory`. Combat still needs a spotted observation to aim, and a **fresh** one to fire.
