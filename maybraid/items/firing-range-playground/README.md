# Firing range

Braidman on a flat range, plus a standing NPC braidman down-left of the pad
body. Both hold a bullpup through [`firearm-user`](../firearm-user/). The
followed player fires from the pad. The NPC installs
[`firearm-intelligence`](../../intelligence/combat/firearm): perception copies
the player into [`FirearmSpotting`](../../intelligence/combat/firearm/src/target.rs);
visible observations feed both firearm objective lists until spotting memory
expires.
Firearm movement writes [`VantageOn`](../../intelligence/movement/lib/src/objective.rs)
/ flee into [`movement-intelligence`](../../intelligence/movement/lib). Firearm
combat aims [`PlayerLook`](../../player/src/identity.rs) from the posed muzzle so
a right-shoulder hold does not walk shots past the target, and fires when the
propagated barrel is aligned onto the capsule.
Projectile sweeps include both fixed geometry and animated character capsules.
Each character starts with 100 health and takes 25 damage per bolt contact.
Health is shown on a persistent top HUD and as a bar above each capsule; the
player also gets directional hit ticks around screen center. At 0 health the
combatant and held firearm despawn; the NPC returns after two seconds.

```bash
cargo run -p firing-range-playground
```

WASD / left stick move, mouse / right stick look, Space / A jump, click / RT fire. R3 toggles first person; right mouse / LT focuses from the head camera onto the firearm sight. `/` then `pause` / `resume`.
