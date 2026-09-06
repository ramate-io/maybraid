# World

Assembled world model. Streamed forest at 1 km grove fill / 3 km selection
generate, with canopy bump-outs in the 1–5 km present keep (spawned only where
160 m fine cells already exist). Fine terrain is the [#675](https://github.com/ramate-io/maybraid/pull/675)
~2.6 km disk plus short 2×/4× macro rings. Character mode is the default, with a
soft sky-dome wash from 350 m to 1200 m. The world uses the shared player camera
and firearm presentation stack: R3 / look-stick click toggles first person,
focus blends onto the firearm sight and its FOV, and the third-person camera
shapecasts against Fixed terrain and vegetation sticks so it does not clip
through. Outgoing hits show FFA-style score markers, and incoming damage shows
directional indicators; health bars remain hidden. Vegetation continues to own
the player capsule locomotion.

```bash
cargo run -p maybraid-world-playground
```

In-game: `/` console, `Y` or `F1` drawer. Character: WASD, mouse look, Space jump.
`mode free` restores the fly camera. FPS is on-screen (toggle with `stats fps`).

A/B the forest against tiled groves in
`chico-vegetation-on-terrain-playground` (`/forest` vs `/grove`).
