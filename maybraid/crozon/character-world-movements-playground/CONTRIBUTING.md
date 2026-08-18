# Contributing to the character world-movements playground

Small Durham patch for iterating Crozon locomotion (walk / run / jump, facing,
grounding) on real ground. Visual pitch samples **Avian colliders**
(`ground-avian`), not the Durham heightfield. Concepts sliders stay in
[`character-concepts-playground`](../character-concepts-playground/). The large
terrain viewer stays in
[`durham/models-playground`](../../durham/models-playground/).
Motion dataflow: [`crozon-character-motion`](../character-motion/README.md).

## Layout

- 4×4 fine cells, no macro rings (~640 m at `TERRAIN_CELL_SIZE`)
- Highest mesh band (`res_2 = 5`) across the patch
- Default spawn is clothed braidman in character mode

## Run

```bash
cargo run -p crozon-character-world-movements-playground
cargo run -p crozon-character-world-movements-playground -- set-character mygr
cargo run -p crozon-character-world-movements-playground -- stampede
```

In-game: `/` console, `Y` or `F1` drawer.

- `set-character <species>`
- `stampede` — every biped and quadruped on its own capsule, 4 m grid, same WASD / jump events. Forelimbed species are omitted. Use `mode free` to orbit the pack.
- `mode free|character`
- WASD move, mouse look, Space jump

## Verify

```bash
cargo check -p crozon-character-world-movements-playground
```
