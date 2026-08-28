# Sky

Distance-fade sky dome. An inverted sphere follows the camera so far terrain
and forest wash to blue. This is an aesthetic mask, not a cull clock.

Default wash: haze starts at 350 m XZ, peaks at 32% alpha by 1200 m. The
shell itself is 2800 m so it stays off the near ground.

```bash
# composed into the world playground
cargo run -p maybraid-world-playground
```
