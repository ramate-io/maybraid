# Firing range

Braidman on a flat range. Pad / WASD drives the character controller. The held bullpup fires on right trigger or left click.

```bash
cargo run -p firing-range-playground
```

WASD / left stick move, mouse / right stick look, Space / A jump, click / RT fire. R3 toggles first person; right mouse / LT focuses from the head camera onto the firearm sight. `/` then `pause` / `resume`.

The held bullpup pins its authored `stock` socket to the trigger shoulder. Yaw tracks look within the configured body-facing limit; pitch matches the camera. Kit meshes are meter-authored and uniformly scaled down to ~0.7 m.

After walk/run, `sync_hands_to_firearm` replaces the full locomotion arm pose with two-bone reaches toward `trigger_point` / `grip_point`. The reticle is an emissive world marker at the first fixed hit along the firearm barrel.
