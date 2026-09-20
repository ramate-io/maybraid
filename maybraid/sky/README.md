# Sky

Distance-fade sky dome. An inverted sphere follows the camera so far terrain
and forest wash to blue. This is an aesthetic mask, not a cull clock.

Default wash: haze starts at 350 m XZ, peaks at 32% alpha by 1200 m. The
shell itself is 2800 m so it stays off the near ground.

The sun's cascade shadows are [`ShadowQuality`](lib/src/shadows.rs): High
(Bevy default, four maps to 150 m), Low (four maps to 60 m, 512²), or Off.
Cascade count stays at four so cycling quality does not trip Bevy 0.19's
stale per-thread visibility queues. The in-game Settings menu cycles that
resource.

```bash
# composed into the world playground
cargo run -p maybraid-world-playground
```
