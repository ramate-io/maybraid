# Maybraid VFX Playground

Isolated host for [`maybraid-vfx`](../lib). Spawn the composed fiery explosion, the God Beam column, sonic waves, or any single burst layer.

```text
cargo run -p vfx-playground -- fiery-explosion
cargo run -p vfx-playground -- show fireball
cargo run -p vfx-playground -- fiery-explosion --scale 1.5 --intensity 1.2 --seed 4 --playback 0.5
cargo run -p vfx-playground -- god-beam --width 7 --lifetime 10 --seed 2
cargo run -p vfx-playground -- sonic-wave --range 20 --speed 22 --count 3 --radius 0.55
```

In-game (press `/`):

- `fiery-explosion` — all four layers (flash, fireball, smoke, sparks)
- `god-beam` — 7 m diameter heavenly column, 10 s active. `--width` is diameter in meters
- `sonic-wave` — mouth-emitted vortex rings. `--range` / `--speed` / `--radius` / `--count` / `--interval`. A tiny driver writes `VfxWaveStates`; the effect does not simulate travel.
- `show` — same as `fiery-explosion`
- `show flash|fireball|smoke|sparks|god-beam|sonic-wave` — one definition
- `--distance` — offset along +Z; `spread` places near / mid / far
- `--seed` — reproduce an instance; omit to generate a new one
- `--playback` — 1.0 is authored tempo; `0.5` is half speed, `2` is twice as fast
- `cancel` — fade out live instances
- `orbit` — orbit the camera around the burst
- `freeze 0.4` — pause once the instance reaches that age; `play` resumes
- `noloop` — stop repeating the last show
- `hud` / `F1` — toggle the bottom command drawer (hide it for captures)
- The last effect loops so a one-shot burst can be judged

The scene includes a warm reference ground, an occluding wall, a box, and a 1.8 m character-scale stand-in just outside a 7 m God Beam. Sonic-wave shows add a mouth-height placeholder head (boss-sized when range ≥ 16 m or radius ≥ 0.7 m). God Beam pulls the camera back so the full column fits; sonic-wave defaults to a 3/4 view. `VFX_FRAME=base` is the God Beam close-up; `VFX_FRAME=side` is a true side view of traveling rings.

Evidence stills: set `VFX_CAPTURE=1` (hides the drawer, disables looping). Optional `VFX_FREEZE=<age>`, `VFX_CANCEL_AT=<age>`, `VFX_NOLOOP=1`, `VFX_FRAME=full|base|side|three-quarter`. See [`evidence/`](evidence/README.md).
