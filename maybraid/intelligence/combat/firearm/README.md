# Firearm intelligence

Higher-order firearm combatant brains.

- [`spotting-intelligence`](../../spotting/lib) owns semantic character discovery, explicit
  subject hints, bounded eye probes, respot cadence, and visibility memory. Its Avian backend
  merges hinted subjects with `Animated` broadphase results, then tests capsule samples against
  Fixed geometry.
- [`combat-targeting`](../targeting) owns remembered combat contacts and the objective-scoped active set. Firearm spotting contributes contact, hostility, and distance bias only after enemyship, an objective, received fire, or another semantic source admits the subject; seeing an allied character does not make it shootable. The per-user algebra combines those inputs with threat, opportunity, continuity, uncertainty, and temporary decaying influences into a cached descending rank.
- [`FirearmIntelligence`](src/combat.rs) owns accuracy, angular tracking, motion-tracking latency, recoil recovery, headshot preference, trigger policy, and fire freshness. It follows the ranked combat target from the stock pose. Lasers and automatic/gated firearms hold through short alignment interruptions; semi-automatic firearms pulse rising edges and bursts release to rearm after their rounds are consumed. [`FirearmEngagement`](src/engagement.rs) is the per-combatant weapons grant: hold-fire, return-fire against recorded attackers, or weapons-free. Absence of the component is weapons-free. [`FirearmTargeting`](src/targeting.rs) separately samples body/head trajectories from the fully posed muzzle, caches them while endpoints and the observation remain unchanged, and contributes clear-shot opportunity back to the next targeting rank.
- [`FirearmMovementIntelligence`](src/movement.rs) reads the same weighted combat list and writes range, cover, flee, and vantage policy into generic movement intelligence. Stale sight lowers hide and raises sightline weight so the mover searches rather than remaining glued to a cover hole.

Respotting currently resolves the subject's exact live location by design; memory controls whether the contact remains actionable, how often it is probed, and when it is forgotten. Eye spotting runs in `Update`. Muzzle validation runs in `PostUpdate` after firearm pose and transform propagation, and fire consumes only a fresh trajectory under its obstruction policy.
