# World

Assembled world model. Streamed forest at 2 km present / 3 km generate.
Fine terrain covers that generate ring; short 2×/4× macro rings sit outside
it. Character mode is the default, with a soft sky-dome wash from 350 m to
1200 m. R3 / look-stick click toggles an experimental first-person view
(the orbit, collision, and POV toggle should move to a shared follow-cam
crate later). The third-person camera shapecasts against Fixed terrain and
vegetation sticks so it does not clip through.

```bash
cargo run -p maybraid-world-playground
```

In-game: `/` console, `Y` or `F1` drawer. Character: WASD, mouse look, Space jump.
`mode free` restores the fly camera. FPS is on-screen (toggle with `stats fps`).

A/B the forest against tiled groves in
`chico-vegetation-on-terrain-playground` (`/forest` vs `/grove`).
