# POI intelligence playground

Run:

```sh
cargo run -p poi-playground --release
```

The flat map contains one local POI per 256 m tile. Sparse gold and pink pillars
are also global POIs. Four colored capsules compare:

- weighted nearby meandering;
- a fixed four-POI meandering cycle;
- weighted distant-tile journeying;
- a fixed four-POI journeying cycle.

Colored lines show active goals. The HUD reports learned POI count, cycle roster
size, goal generation, and target. Press Space to pause or resume movement.

The playground intentionally realizes movement as a direct flat-world walk. It
isolates POI discovery and selection behavior from terrain probing and route
quality; the reusable `PoiGoal` handoff remains the same one consumed by routing
or movement in a game stack.
