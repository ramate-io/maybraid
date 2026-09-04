# Routing playground

Durham fine-grid terrain (same 4×4 / ~640 m patch as vegetation-on-terrain,
without groves). A magenta NPC tethers to the tan player capsule by default
(or stalks within an annulus around them). Far remaining work uses hierarchical
routing; close remaining work uses local movement.

Fly camera is the default: **WASD**, mouse look, **Space** up / **Shift** down
(starts in a near-overhead fixture view so the player, NPC, and initial route
are legible). `Y` or `F1` opens the command drawer. `/mode character` is a
third-person capsule.

Orange / yellow / cyan gizmos are coarse → fine corridors (playground bands
**160 / 80 / 32 m**). The white sphere is the current routing destination.
Band lengths live on [`RoutingSettings`](../routing/src/band.rs), not in the
crate.

```bash
cargo run -p routing-playground --release
```

```
/tether 8
/stalk 8 12
/idle
/drive
/go 220 48
/mode character
/help
```
