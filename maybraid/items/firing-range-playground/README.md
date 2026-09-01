# Firing range

Third-person Brodler on a flat range. Pad / WASD drives the character controller. The held bullpup fires on right trigger or left click.

```bash
cargo run -p firing-range-playground
```

WASD / left stick move, mouse / right stick look, Space / A jump, click / RT fire. `/` then `pause` / `resume`.

The held bullpup is posed from the rig: between `shoulder.L` / `shoulder.R`, then forward in proportion to arm reach. Yaw tracks look within ±30° of body facing; pitch matches the camera. Kit meshes are meter-authored and uniformly scaled down to ~0.7 m.

After walk/run, `sync_hands_to_firearm` replaces the full locomotion arm pose with stable uppercut-style reaches toward `trigger_point` / `grip_point`. The reticle raycasts along the firearm bore and projects the hit as always-visible UI.
