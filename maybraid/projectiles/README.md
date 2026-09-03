# Projectiles

Flight, penetration, and first-contact for query-only bolts and bullets.

Weapons (muzzle, cooldown, lasers) stay in item crates. [`tick_flights`](src/lib.rs)
shapecasts each flight in ≤2 m segments so fast bolts still register thin
colliders. Visual impacts listen to [`ProjectileContact`](src/lib.rs); this crate
does not depend on Hanabi or firearms.

```text
Weapon fire  →  spawn_flight
tick_flights →  Flight budgets + ProjectileContact
item VFX     →  consume contact
```
