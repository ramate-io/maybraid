# Maybraid Game

The actual Maybraid game.

Starts on the home screen. Discovery enters the streamed world. Training Ground
seats the saved character on a pinned FinePatch of that same stack. Characters opens the
character screen. Reliquary is not wired yet. In world, Start (Enter on
keyboard) overlays the in-game menu; Start again dismisses it. Leave on that
menu returns home. Settings opens the same pause settings overlay (shadows, mob
HUD); Back / Escape returns home.

```bash
cargo run -p maybraid
```

Assets live in this crate’s `assets/` symlink (`maybraid/assets`). A packaged
`.app` reads `Contents/Resources/assets` instead; see [packaging/](../../packaging/).

```bash
packaging/scripts/package-macos.sh
open dist/Maybraid.app
```
