# Routing playground

Durham fine-grid terrain (same 4×4 / ~640 m patch as vegetation-on-terrain,
without groves) with one magenta NPC walking a hierarchical route.

Fly camera is the default: WASD, mouse look, Space / Shift vertical. `Y` or
`F1` opens the command drawer. `/mode character` is a third-person capsule.

Orange / yellow / cyan gizmos are coarse → fine corridors (playground bands
**160 / 80 / 32 m**). The white sphere is the destination. Band lengths live
on [`RoutingSettings`](../routing/src/band.rs), not in the crate.

```bash
cargo run -p routing-playground --release
```

```
/go 220 48
/mode free
/mode character
/help
```
