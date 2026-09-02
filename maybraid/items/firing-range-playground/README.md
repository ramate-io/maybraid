# Firing range

Braidman on a flat range, plus a standing NPC braidman down-left of the pad
body. Both hold a bullpup through [`firearm-user`](../firearm-user/). The
followed player fires from the pad. The NPC installs
[`firearm-intelligence`](../../intelligence/combat/firearm): perception copies
the player into [`FirearmSpotting`](../../intelligence/combat/firearm/src/target.rs);
visible observations feed both firearm objective lists until spotting memory
expires.
Firearm movement writes [`VantageOn`](../../intelligence/movement/lib/src/objective.rs)
/ flee into [`movement-intelligence`](../../intelligence/movement/lib); firearm
combat aims [`PlayerLook`](../../player/src/identity.rs) and fires only when the
propagated barrel is aligned and the obstruction policy permits it.

```bash
cargo run -p firing-range-playground
```

WASD / left stick move, mouse / right stick look, Space / A jump, click / RT fire. R3 toggles first person; right mouse / LT focuses from the head camera onto the firearm sight. `/` then `pause` / `resume`.
