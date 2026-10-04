# Maybraid VFX Playground

Isolated host for [`maybraid-vfx`](../lib). Spawn the composed fiery explosion or any single layer.

```text
cargo run -p vfx-playground -- firey-explosion
cargo run -p vfx-playground -- show fireball
cargo run -p vfx-playground -- firey-explosion --scale 1.5 --intensity 1.2
```

In-game (press `/`):

- `firey-explosion` — all four layers (flash, fireball, smoke, sparks)
- `show` — same as `firey-explosion`
- `show flash|fireball|smoke|sparks` — one layer
- Repeat a command to overlay concurrent instances
- The last effect loops so a one-shot burst can be judged
