# Firing range

Braidman on a flat range, plus a standing NPC braidman down-left of the pad
body. Both hold a bullpup through [`firearm-user`](../firearm-user/). The
followed player fires from the pad. The NPC installs
[`firearm-intelligence`](../../intelligence/combat/firearm): perception copies
the player into [`FirearmSpotting`](../../intelligence/combat/firearm/src/target.rs)
(and, in free-for-all, every other live combatant);
visible observations feed firearm combat until spotting memory expires.
The NPC traces `vision` capsule samples per frame (default 9) and spends
`focus` of that budget on the highest-ranked target.
Look tracks the live capsule; fire needs that sightline to stay fresh.
Firearm movement hunts those candidates even without a current sightline, then
writes [`VantageOn`](../../intelligence/movement/lib/src/objective.rs)
into [`movement-intelligence`](../../intelligence/movement/lib) at an ~8 m
standoff (no close-range flee). Lost sightlines raise sightline weight so the
NPC does not glue to a cover crack. Firearm combat aims
[`PlayerLook`](../../player/src/identity.rs) from the stock (shoulder pivot), not
the muzzle, and holds the trigger after the first on-target acquire;
`trigger_happiness` is only the delay before that first pull.
Projectile sweeps include both fixed geometry and animated character capsules.
Collider size comes from the character recipe
[`LocomotionCapsule`](../../crozon/characters/src/components.rs) (Braidman is
the 0.4 / 1.0 humanoid hull). Each character starts with 100 health and takes 25 damage per bolt contact
in `duel`. `free-for-all` bakes the rolled firearm's DPC, speed, range,
penetration, cadence, and recoil (plus clothing HP / outgoing damage) into the
live weapon. Health is shown on a persistent top HUD and as a bar above each
capsule; the followed player's gun stats sit in the bottom-right card. The
world reticle flashes when the followed player lands a hit, and `+1` / `+2` /
`+5` float at the impact point for a body hit, a headshot (upper half of the
top capsule hemisphere, 1.25× HP, light blue), or a down. Catalog recoil noisily
kicks the follow camera (and NPC look) along a short lerp to the hashed offset
(scaled by the rolled recoil value); lasers do not kick. Connected pads rumble
on the followed player's fire and hit-confirm (faster / harder shots scale the
pulse; lasers stay a low constant tick). The
player also gets directional hit ticks around screen center. At 0 health the
combatant and held firearm despawn; player and NPC both return after two seconds.
The NPC spots and aims during a ceasefire, but does not fire until the player
takes a shot. Player death (or switching mode) resets that ceasefire.

`free-for-all` is a generated-loadout benchmark: one rolled player (starter
clothing plus one gallery-style firearm from
[`crozon-character-items`](../../crozon/character-items)) and `--npcs` rolled
NPCs spread around the pad and on the upper storey, all of whom list every other
combatant as a spotting candidate. Combat
still waits for the player's first shot. Rolled guns keep gallery looks
(material and palette per slot) and sample projectile / cadence from the
session RNG rather than reseeding from spec identity, so a run is not locked
to one color or to lasers. `duel` restores the 1v1 bullpup pad fight
(100 HP / 25 DPC). `test-dummy` (`dummy`) spawns the player with that same
bullpup and a stationary unarmed braidman at the NPC pad pose — no combat AI,
no return fire — so projectile contacts can be checked in isolation.

```bash
cargo run -p firing-range-playground
cargo run -p firing-range-playground -- free-for-all --npcs 8
cargo run -p firing-range-playground -- test-dummy
```

WASD / left stick move, mouse / right stick look, Space / A jump, click / RT fire. R3 toggles first person; right mouse / LT focuses from the head camera onto the firearm sight. `/` then `pause` / `resume` / `free-for-all` / `duel` / `test-dummy`.
