# Richmond Buildings Playground

Interactive viewer for Richmond building components and the Wizard's Tower sketch.

## Run

```bash
cargo run -p richmond-buildings-playground
# or with a startup command:
cargo run -p richmond-buildings-playground -- show linear
cargo run -p richmond-buildings-playground -- show arc-90
cargo run -p richmond-buildings-playground -- show wizards-tower --noise 0.4
```

In-game: press `/` for the command console (same clap commands as argv).

## Commands

- `help`
- `show linear|arc-90|arc-180|header-90` — partition leaves with GLBs under `urban/partitions/rough_stonework/`
- `show wizards-tower [--noise 0.5]` — authored tower hierarchy (`LodScene` composition)

WASD / Space / Shift + mouse look.
