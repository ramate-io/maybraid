# Damage

Generic hit receiver. Producers write [`Hit`](src/lib.rs); this crate mutates
[`Health`](src/lib.rs) and emits [`DamageApplied`](src/lib.rs) / [`Died`](src/lib.rs).
`Died` is materialized once as a durable [`Downed`](src/lifecycle.rs) component.
[`DespawnAfter`](src/lifecycle.rs) provides a shared game-time queue so consumers
can finish visual or event handoffs before removing an entity hierarchy.

[`projectiles`](../projectiles/) stay geometry-only. A contact adapter copies
[`HitPayload`](src/lib.rs) off the projectile into `Hit`. An optional
[`HeadshotBand`](src/lib.rs) on the target scales that amount when the contact
sits above `min_local_y` in the target's local Y. Firearms, melee, and
playgrounds do not apply HP themselves; they stamp payload and (if they want a
bonus) the band.

```text
ProjectileContact + HitPayload  →  Hit
Hit + optional HeadshotBand     →  Health, DamageApplied, Died → Downed
```
