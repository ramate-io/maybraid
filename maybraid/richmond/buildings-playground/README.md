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
cargo run -p richmond-buildings-playground -- show bedroom --extent 8,3,8 --occupancy 0.8 --spaciousness 1.2
# Rectangular pitch (no end triangles):
cargo run -p richmond-buildings-playground -- show pitch --rise 1 --run 2 --length 6 --tile-width 1
# Ridge longer than eave (flipped ends via from_eave_ridge):
cargo run -p richmond-buildings-playground -- show pitch --rise 1 --run 2 --eave 4 --ridge 6 --tile-width 1
# Explicit ends (positive = upright / eave-long, negative = flipped / ridge-long):
cargo run -p richmond-buildings-playground -- show pitch --rise 1 --run 2 --length 4 --left 1 --right -0.5
```

In-game: press `/` for the command console (same clap commands as argv).

## Commands

- `help`
- `show linear|arc-90|arc-180|header-90` — partition leaves with GLBs under `urban/partitions/rough_stonework/`
- `show pitch [--rise R] [--run R] [--length L] [--tile-width W] [--left B] [--right B] | [--eave E --ridge R]` — shepherd's-thatch pitched face; omit `--length`/`--left`/`--right` for optional regions; `--eave`+`--ridge` uses equal end triangles
- `show wizards-tower [--noise 0.5]` — authored tower hierarchy (`LodScene` composition)
- `show stacked-rings [--floor-count N] [--floor-height H] [--radius R]` — circular wall stack (kit scale check)
- `show bedroom [--extent X,Y,Z] [--noise 0.5] [--spaciousness 1.0] [--occupancy 0.55] [--door]` — hierarchical bedroom; bed-first multi-fill under spaciousness/occupancy; `--door` adds a −Z circulation exclusion

WASD / Space / Shift + mouse look.
