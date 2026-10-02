# Mob on terrain

A small Durham fine-grid patch (same 4×4 / ~640 m as routing-playground) with a
**short authored mob list**. Use this when the world stream is too noisy to tell
whether a host is sitting on composed height, riding a bad routing hop, or
dragging High plants through the sky.

There is no vegetation and no 400 m mob LOD stream. Hosts spawn only after
composed height and a terrain trimesh exist. Journeying still uses the production
stack: the host is a corridor planner (`RoutingIntelligenceUser`); `MobTravel`
slides it along hops (including Y); High plants follow by tether, not by
`ChildOf` the moving host.

Capsules use the world steep-hill setup from
[#718](https://github.com/ramate-io/maybraid/pull/718): 70° walkable slope,
terrain friction 2.55/2.95 Max (static > tan(70°) ≈ 2.75), and grounded wish
projected onto the contact plane so XZ accel does not dig into the hill. Default
Durham trimesh grip (0.95 static) only holds ~44°. Quadruped visuals get the
same Avian terrain-pitch apply as vegetation-on-terrain (`face_wish` is XZ-only).

Default cast is a **herd** (≤6 members). `/pack` and `/both` swap the list.
`/hars`, `/ylter`, and `/hars-ylter` force those quadruped species onto a
short herd (2 plants each) so terrain pitch can be compared without the world
stream. Playground herds use an 80 m leash (pack 48 m) copied onto member
tethers after bind — grazer personality still defaults to 24 m. Host
journeying uses 48 m tiles and `min_tile_distance = 1` so the 90 m forage ring
is a neighboring cell. Yellow spheres are `GlobalPoi` forage; smaller
blue/green markers are `LocalPoi` snacks inside the meander radius.

```bash
cargo run -p mob-on-terrain-playground --release
cargo run -p mob-on-terrain-playground --release -- pack
cargo run -p mob-on-terrain-playground --release -- both
cargo run -p mob-on-terrain-playground --release -- hars-ylter
```

Fly camera is the default: **WASD**, mouse look, **Space** up / **Shift** down.
`Y` or `F1` opens the command drawer.

```
/herd
/pack
/both
/hars
/ylter
/hars-ylter
/rebuild
/mode character
/help
```

Status text reports host Y vs composed terrain Y, current hop Y, destination Y,
and High-plant Y range. Magenta ground lines mean the host is more than 2 m off
the sampled surface. Orange / yellow / cyan gizmos are routing corridors.
Cyan spheres are plants.
