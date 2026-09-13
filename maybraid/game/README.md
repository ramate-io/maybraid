# Maybraid Game

The actual Maybraid game.

Starts on the home screen. Discovery and Reliquary enter the world playground.
Characters opens the character screen. In world, Start (Enter on keyboard)
overlays the in-game menu; Start again dismisses it. Leave on that menu returns
home. Training Ground and Settings are not wired yet.

```bash
cargo run -p maybraid
```

Assets live in this crate’s `assets/` symlink (`maybraid/assets`). A packaged
`.app` reads `Contents/Resources/assets` instead; see [packaging/](../../packaging/).

```bash
packaging/scripts/package-macos.sh
open dist/Maybraid.app
```
