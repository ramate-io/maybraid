# Maybraid VFX Playground

Isolated host for [`maybraid-vfx`](../lib). Spawn the composed fiery explosion or any single layer.

```text
cargo run -p vfx-playground -- firey-explosion
cargo run -p vfx-playground -- show fireball
cargo run -p vfx-playground -- firey-explosion --scale 1.5 --intensity 1.2 --seed 4
```

In-game (press `/`):

- `firey-explosion` — all four layers (flash, fireball, smoke, sparks)
- `show` — same as `firey-explosion`
- `show flash|fireball|smoke|sparks` — one layer
- `--distance` — offset along +Z; `spread` places near / mid / far
- `--seed` — independent instance variation
- `orbit` — orbit the camera around the burst
- `freeze 0.4` — pause once the instance reaches that age; `play` resumes
- `noloop` — stop repeating the last show
- The last effect loops so a one-shot burst can be judged

The scene includes a warm reference ground, an occluding wall, and a box so orbiting can show depth and occlusion.
