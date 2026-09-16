# Sky

Distance-fade wash plus an outer Cosimo-like field. An inverted sphere
follows the camera so far terrain washes toward the horizon. This is an
aesthetic mask, not a cull clock.

Default wash: haze starts at 350 m XZ, peaks at 32% alpha by 1200 m. The
inner shell is 2800 m. Vertex alpha still follows XZ; hue is tinted from
[`SkyClock`](lib/src/clock.rs). Overhead is the outer field shader
([`field.wgsl`](lib/src/field.wgsl)): a zenith→horizon gradient, slow
chroma breathe, nebula swirls, and star glints gated by day weight.

The default clock is **paused at golden** (phase `0.62`, the authored
−45°/45° amber key). `/sky play` or `MAYBRAID_SKY_PLAY=1` starts a 30
minute cycle. Presets: `dawn`, `morning`, `noon`, `golden`, `dusk`,
`night`. Also `MAYBRAID_SKY_PHASE` and `MAYBRAID_SKY_RATE`.

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
