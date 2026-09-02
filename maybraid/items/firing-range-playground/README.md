# Firing range

Braidman on a flat range, plus a standing NPC braidman down-left of the pad
body. Both hold a bullpup through [`firearm-user`](../firearm-user/); only the
followed player fires and drives the reticle. The NPC uses
[`movement-intelligence`](../../intelligence/lib) with an Avian collider
surface: the playground writes [`VantageOn`](../../intelligence/lib/src/objective.rs)
the player and requests a replan when the player has moved.

The playground registers player, camera, firearm-user, weapons, and movement
intelligence plugins; the pad, cover crates, and vantage refresh stay here.

```bash
cargo run -p firing-range-playground
```

WASD / left stick move, mouse / right stick look, Space / A jump, click / RT fire. R3 toggles first person; right mouse / LT focuses from the head camera onto the firearm sight. `/` then `pause` / `resume`.
