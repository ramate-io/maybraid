# Maybraid Game

The actual Maybraid game.

Starts on the home screen. Discovery and Reliquary enter the world playground.
Characters opens the character screen. In world, Start (Enter on keyboard)
overlays the in-game menu; Start again dismisses it. Leave on that menu returns
home. Training Ground is not wired yet. Settings opens the same pause
settings overlay (shadows, pixel count, mob HUD); Back / Escape returns home.
Pixel count scales the 3D main pass; the window stays native.

```bash
cargo run -p maybraid
```

Assets live in this crate’s `assets/` directory (Bevy’s usual layout).
