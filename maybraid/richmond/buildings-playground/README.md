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
cargo run -p richmond-buildings-playground -- show tessellated-triangle
cargo run -p richmond-buildings-playground -- show tessellated-triangle --a 0,0 --b 3,0 --c 0,2
# Folded quad (default ~90° crease + joint):
cargo run -p richmond-buildings-playground -- show quad-panel
# Coplanar quad (no joint):
cargo run -p richmond-buildings-playground -- show quad-panel --a0 0,0,0 --a1 3,0,0 --b0 0,0,3 --b1 3,0,3
# Panel complex (default mild trapezoid); compact mesh syntax:
cargo run -p richmond-buildings-playground -- show panel-complex
cargo run -p richmond-buildings-playground -- show panel-complex \
  --mesh '1=(0,0,0),2=(1,0,0),3=(0,1,0),4=(0,0,1) ... {1,2,4},{1,4,3}'
# Same geometry as one quad face:
cargo run -p richmond-buildings-playground -- show quad-panel-complex \
  --mesh '1=(0,0,0),2=(1,0,0),3=(0,1,0),4=(0,0,1) ... {1,2,3,4}'
# Ruled pitch strip (default funky 5+5 eave/ridge):
cargo run -p richmond-buildings-playground -- show ruled-pitch
```

In-game: press `/` for the command console (same clap commands as argv).

## Commands

- `help`
- `show linear|arc-90|arc-180|slice-90` — partition leaves (`panels/.../rectangle_001`, `arcs/.../arc_*`)
- `show pitch [--rise R] [--run R] [--length L] [--tile-width W] [--left B] [--right B] | [--eave E --ridge R]` — shepherd's-thatch pitched face; omit `--length`/`--left`/`--right` for optional regions; `--eave`+`--ridge` uses equal end triangles
- `show tessellated-triangle [--a X,Z] [--b X,Z] [--c X,Z]` — rough-stone floor fill of a panel-space triangle
- `show quad-panel [--a0|--a1|--b0|--b1 X,Y,Z] [--t-a0|--t-a1|--t-b0|--t-b1 T] [--min-dihedral R] [--no-joint]` — two lines → two tessellated triangles + optional crease `JointNode` (default corners are a ~90° fold; thicknesses default to 0.4)
- `show panel-complex [--mesh 'id=(x,y,z) … {a,b,c}'] [--min-dihedral R] [--no-joint]` — point-id triangle mesh + crease joints; optional thickness as 4th tuple component
- `show quad-panel-complex [--mesh 'id=(x,y,z) … {a0,a1,b0,b1}'] [--min-dihedral R] [--no-joint]` — quad-face mesh (diagonal a0–b1) → same presentation path
- `show ruled-pitch [--min-dihedral R] [--no-joint]` — `RuledPitch` (eave/`rail_a`, ridge/`rail_b`; default funky 5+5) → ruled quad strip + crease joints
- `show wizards-tower [--noise 0.5]` — authored tower hierarchy (`LodScene` composition)
- `show stacked-rings [--floor-count N] [--floor-height H] [--radius R]` — circular wall stack (kit scale check)
- `show bedroom [--extent X,Y,Z] [--noise 0.5] [--spaciousness 1.0] [--occupancy 0.55] [--door]` — hierarchical bedroom; bed-first multi-fill under spaciousness/occupancy; `--door` adds a −Z circulation exclusion

WASD / Space / Shift + mouse look.
