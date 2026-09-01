# Firing range

Third-person Brodler on a flat range. Pad / WASD drives the character controller. The held bullpup fires on right trigger or left click.

```bash
cargo run -p firing-range-playground
```

WASD / left stick move, mouse / right stick look, Space / A jump, click / RT fire. `/` then `pause` / `resume`.

The gun is not parented to the hand (that inherited the forearm scale). Each frame it copies `forearm.R`'s world translation, uses the player's facing for yaw, and the camera pitch for pitch. Walk/run still play; a hold overlay then overwrites both arms.
