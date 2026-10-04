# Maybraid VFX Playground

Isolated host for [`maybraid-vfx`](../lib). Spawn the composed fiery explosion or any single layer.

```text
cargo run -p vfx-playground
cargo run -p vfx-playground -- show firey-explosion
cargo run -p vfx-playground -- show sparks --scale 2 --intensity 1.5
```

In-game (press `/`):

- `show` — `firey-explosion` at the origin
- `show flash|fireball|smoke|sparks`
- `show firey-explosion --scale 1.5 --intensity 1.2`
- Repeat `show` to overlay concurrent instances
