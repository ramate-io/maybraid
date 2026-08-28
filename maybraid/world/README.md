# World

Assembled world model. Terrain + streamed forest at playable extents
(2 km present / 3 km generate), character mode by default, with a soft
sky-dome wash from 350 m to 1200 m.

```bash
cargo run -p maybraid-world-playground
```

In-game: `/` console, `Y` or `F1` drawer. Character: WASD, mouse look, Space jump.
`mode free` restores the fly camera. FPS is on-screen (toggle with `stats fps`).

A/B the forest against tiled groves in
`chico-vegetation-on-terrain-playground` (`/forest` vs `/grove`).
