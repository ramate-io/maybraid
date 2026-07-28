# Richmond Buildings Playground

Interactive viewer for Richmond building components and authored buildings.

## Run

```bash
cargo run -p richmond-buildings-playground
# or with a startup command:
cargo run -p richmond-buildings-playground -- show linear
cargo run -p richmond-buildings-playground -- show arc-90
cargo run -p richmond-buildings-playground -- show wizards-tower --noise 0.4
cargo run -p richmond-buildings-playground -- show stacked-rings --floor-count 6 --floor-height 2.5 --radius 3
cargo run -p richmond-buildings-playground -- show bedroom
cargo run -p richmond-buildings-playground -- show bedroom --extent 6,2.8,4 --noise 0.2 --door
```

In-game: press `/` for the command console (same clap commands as argv).

## Commands

- `help`
- `show linear|arc-90|arc-180|header-90` — partition leaves with GLBs under `urban/partitions/rough_stonework/`
- `show wizards-tower [--noise 0.5]` — authored tower hierarchy (`LodScene` composition)
- `show stacked-rings [--floor-count N] [--floor-height H] [--radius R]` — circular wall stack (kit scale check)
- `show bedroom [--extent X,Y,Z] [--noise 0.5] [--door]` — hierarchical bedroom; `--door` adds a −Z circulation exclusion for layout fitting

WASD / Space / Shift + mouse look.
