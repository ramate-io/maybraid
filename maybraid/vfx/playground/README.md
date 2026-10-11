# Maybraid VFX Playground

Isolated host for [`maybraid-vfx`](../lib). Spawn the composed fiery explosion, the God Beam column, sonic waves, or any single burst layer.

```text
cargo run -p vfx-playground -- fiery-explosion
cargo run -p vfx-playground -- show fireball
cargo run -p vfx-playground -- fiery-explosion --scale 1.5 --intensity 1.2 --seed 4 --playback 0.5
cargo run -p vfx-playground -- god-beam --width 7 --lifetime 10 --seed 2
cargo run -p vfx-playground -- sonic-wave --range 28 --speed 22 --count 3
```

In-game (press `/`):

- `fiery-explosion` — all four layers (flash, fireball, smoke, sparks)
- `god-beam` — 7 m diameter heavenly column, 10 s active. `--width` is diameter in meters
- `sonic-wave` — propagating rings. `--range` / `--speed` / `--count` should match B04 wavefronts
- `show` — same as `fiery-explosion`
- `show flash|fireball|smoke|sparks|god-beam|sonic-wave` — one definition
- `--distance` — offset along +Z; `spread` places near / mid / far
- `--seed` — reproduce an instance; omit to generate a new one
- `--playback` — 1.0 is authored tempo; `0.5` is half speed, `2` is twice as fast
- `cancel` — fade out live instances
- `orbit` — orbit the camera around the burst
- `freeze 0.4` — pause once the instance reaches that age; `play` resumes
- `noloop` — stop repeating the last show
- The last effect loops so a one-shot burst can be judged

The scene includes a warm reference ground, an occluding wall, and a box so orbiting can show depth and occlusion. God Beam and sonic-wave shows pull the camera back to boss-scale viewing distance.
