# Firing range

Braidman on a flat range, plus a standing NPC braidman to the camera-right of
the pad body. Both hold a bullpup through [`firearm-user`](../firearm-user/);
only the followed player fires and drives the reticle.

The playground registers player, camera, firearm-user, and weapons plugins; the
pad and range geometry stay here.

```bash
cargo run -p firing-range-playground
```

WASD / left stick move, mouse / right stick look, Space / A jump, click / RT fire. R3 toggles first person; right mouse / LT focuses from the head camera onto the firearm sight. `/` then `pause` / `resume`.
