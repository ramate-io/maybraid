# Maybraid VFX Playground

Isolated host for [`maybraid-vfx`](../lib). Spawn the composed fiery explosion or any single layer.

```text
cargo run -p vfx-playground -- fiery-explosion
cargo run -p vfx-playground -- show fireball
cargo run -p vfx-playground -- show dust --playback 0.5
cargo run -p vfx-playground -- fiery-explosion --scale 1.5 --intensity 1.2 --seed 4 --playback 0.5
```

In-game (press `/`):

- `fiery-explosion` — all four layers (flash, fireball, smoke, sparks)
- `show` — same as `fiery-explosion`
- `show flash|fireball|smoke|sparks|dust` — one layer
- `--distance` — offset along +Z; `spread` places near / mid / far
- `--seed` — reproduce an instance; omit to generate a new one
- `--playback` — 1.0 is authored tempo; `0.5` is half speed, `2` is twice as fast
- `orbit` — orbit the camera around the burst
- `freeze 0.4` — pause once the instance reaches that age; `play` resumes
- `pause` — freeze all current instances immediately; `play` resumes
- `reset` — cancel current instances and replay the last request from frame zero
- `cancel` — remove all instances and pending/repeating playback
- `noloop` — stop repeating the last show
- The last effect loops so a one-shot burst can be judged

The scene includes a warm reference ground, an occluding wall, and a box so orbiting can show depth and occlusion.

`show dust` selects the baked rolling-wave dust preset. It spawns at the ground pivot;
its 0.55 m capture-center offset is part of the asset. Use `freeze 0.8` and `orbit`
to inspect its shape, or `spread` / repeated `show dust` commands to compare concurrent
instances. Each instance has its own clock and material uniforms; the eight-view
KTX2 texture is shared. `--scale` and `--playback` apply through the same effect API
as the other presets. Playback ends at the last authored timestamp, 3.2 seconds.

The dust interpolates adjacent times and neighboring azimuth captures in premultiplied
linear color. Its views were authored at 20° elevation: orbiting horizontally is the
intended inspection path, and steep overhead/underside views remain an approximation.
The renderer projects two fixed capture planes, not a simulated volume. The playground
camera uses Bevy's `DepthPrepass`: each plane is rejected against opaque scene depth
before angular blending, with the original 0.18 m contact fade. Other hosts should add
`DepthPrepass` to their camera for this behavior. Without it, the material falls back to
one alpha-weighted hardware depth for the blended pair; intersecting geometry can then
occlude the pair incorrectly. The shared camera-facing proxy can clip at very close
camera distances. It casts no shadow and does not react to scene lighting or obstacles.

A new instance waits for its manifest and image dependencies before advancing. GPU
pipeline compilation is asynchronous; a cold first show can still miss visible opening
frames while its pipeline warms up. Replay with `reset` after the first load when
comparing a particular frame; no guessed startup delay is applied.
