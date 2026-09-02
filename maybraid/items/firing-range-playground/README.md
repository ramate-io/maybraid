# Firing range

Braidman on a flat range, plus a standing NPC braidman down-left of the pad
body. Both hold a bullpup through [`firearm-user`](../firearm-user/). The
followed player fires from the pad. The NPC installs
[`firearm-intelligence`](../../intelligence/combat/firearm): perception copies
the player into [`FirearmSpotting`](../../intelligence/combat/firearm/src/target.rs);
visible observations feed firearm combat until spotting memory expires.
Look tracks the last visible point; fire needs that sightline to stay fresh.
Firearm movement hunts those candidates even without a current sightline, then
writes [`VantageOn`](../../intelligence/movement/lib/src/objective.rs) / flee
into [`movement-intelligence`](../../intelligence/movement/lib) at an ~8 m
standoff. Lost sightlines raise sightline weight so the NPC does not glue to a
cover crack. Firearm combat aims [`PlayerLook`](../../player/src/identity.rs)
from the posed muzzle and holds the trigger while the barrel is on a freshly
spotted point; `trigger_happiness` is only the delay before that first pull.
Projectile sweeps include both fixed geometry and animated character capsules.
Each character starts with 100 health and takes 25 damage per bolt contact.
Health is shown on a persistent top HUD and as a bar above each capsule; the
player also gets directional hit ticks around screen center. At 0 health the
combatant and held firearm despawn; player and NPC both return after two seconds.

```bash
cargo run -p firing-range-playground
```

WASD / left stick move, mouse / right stick look, Space / A jump, click / RT fire. R3 toggles first person; right mouse / LT focuses from the head camera onto the firearm sight. `/` then `pause` / `resume`.
