# Contributing to the character world-movements playground

Small Durham patch for iterating Crozon locomotion (walk / run / jump, facing,
grounding) on real ground. Concepts sliders stay in
[`character-concepts-playground`](../character-concepts-playground/). The large
terrain viewer stays in
[`durham/models-playground`](../../durham/models-playground/).

## Layout

- 4×4 fine cells, no macro rings (~640 m at `TERRAIN_CELL_SIZE`)
- Highest mesh band (`res_2 = 5`) across the patch
- Default spawn is clothed braidman in character mode

## Run

```bash
cargo run -p crozon-character-world-movements-playground
cargo run -p crozon-character-world-movements-playground -- set-character mygr
```

In-game: `/` console, `Y` or `F1` drawer.

- `set-character <species>`
- `mode free|character`
- WASD move, mouse look, Space jump

## Verify

```bash
cargo check -p crozon-character-world-movements-playground
```
