# Sky

Distance-fade sky dome. An inverted sphere follows the camera so far terrain
and forest wash toward a late-afternoon horizon. This is an aesthetic mask,
not a cull clock.

Default wash: haze starts at 350 m XZ, peaks at 32% alpha by 1200 m. The
shell itself is 2800 m so it stays off the near ground. Vertex hue follows
elevation (cool zenith, peach horizon, darker nadir). Clear color is
[`SKY_CLEAR`](lib/src/lib.rs).

The sun is a warm ~8 klux key at the existing −45°/45° pose, with a cooler
fill and a slightly lifted ambient. An unlit sun disk sits on that axis
just inside the dome; a smaller moon and sparse zenith stars are camera-
parented and do not cast shadows. This is not a time-of-day clock.

The sun's cascade shadows are [`ShadowQuality`](lib/src/shadows.rs): High
(Bevy default, four maps to 150 m), Low (four maps to 60 m, 512²), or Off.
Cascade count stays at four so cycling quality does not trip Bevy 0.19's
stale per-thread visibility queues. The in-game Settings menu cycles that
resource.

```bash
# composed into the world playground
cargo run -p maybraid-world-playground
```
