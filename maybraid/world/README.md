# World

Assembled world model. Streamed forest at 1 km grove fill / 3 km selection
generate, with canopy bump-outs in the 1–5 km present keep (spawned only where
160 m fine cells already exist). Fine terrain is the [#675](https://github.com/ramate-io/maybraid/pull/675)
~2.6 km disk plus short 2×/4× macro rings. Character mode is the default, with a
soft sky-dome wash from 350 m to 1200 m. R3 / look-stick click toggles an
experimental first-person view at the posed nose with face parts hidden (the
orbit, collision, and POV toggle should move to a shared follow-cam crate
later). The third-person camera shapecasts against Fixed terrain and vegetation
sticks so it does not clip through.

```bash
cargo run -p maybraid-world-playground
```

In-game: `/` console, `Y` or `F1` drawer. Character: WASD, mouse look, Space jump.
`mode free` restores the fly camera. FPS is on-screen (toggle with `stats fps`).

Forest / grove A/B that used to live in the vegetation-on-terrain playground is retired; see [PLAYGROUNDS.md](../PLAYGROUNDS.md#retired).
