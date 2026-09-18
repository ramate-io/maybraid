# Sky

Blue / haze atmosphere over an opaque Cosimo field. An inverted sphere
follows the camera. This is an aesthetic mask, not a cull clock.

The inner shell ([`dome.wgsl`](lib/src/dome.wgsl)) is a blending
atmosphere: soft-edged haze islands, sparse pinpoints, and a paper sun
(flat disk plus shards turned along the rim, like the Maybraid icon).
Both shells are icospheres.

The outer shell ([`field.wgsl`](lib/src/field.wgsl)) is Cosimo halfway
flattened, with the same paper sun and moon. No mesh disks.

The default clock is **paused at golden** (phase `0.62`, the authored
−45°/45° amber key). `/sky play` or `MAYBRAID_SKY_PLAY=1` starts a 30
minute cycle. Presets: `dawn`, `morning`, `noon` (overhead, warm key,
teal sky), `golden`, `dusk`, `night` (`0`). Also `MAYBRAID_SKY_PHASE`
and `MAYBRAID_SKY_RATE`.

```text
/sky status
/sky golden
/sky noon
/sky at 0.35
/sky pause
/sky play
/sky rate 1200
```

The sun's cascade shadows are [`ShadowQuality`](lib/src/shadows.rs): High
(Bevy default, four maps to 150 m), Low (two maps to 60 m), or Off. The
in-game Settings menu cycles that resource.

```bash
# composed into the world playground
cargo run -p maybraid-world-playground
```
