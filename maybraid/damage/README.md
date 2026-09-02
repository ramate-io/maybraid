# Damage

Generic hit receiver. Producers write [`Hit`](src/lib.rs); this crate mutates
[`Health`](src/lib.rs) and emits [`DamageApplied`](src/lib.rs) / [`Died`](src/lib.rs).

[`projectiles`](../projectiles/) stay geometry-only. A contact adapter copies
[`HitPayload`](src/lib.rs) off the projectile into `Hit`. Firearms, melee, and
playgrounds do not apply HP themselves.

```text
ProjectileContact + HitPayload  →  Hit
Hit                             →  Health, DamageApplied, Died
```
