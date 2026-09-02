# Intelligence

Movement intelligence crates for Maybraid.

- [`movement-intelligence`](lib) — [`MovementIntelligence`](lib/src/user.rs) on a capsule, objectives, and a [`MovementIntelligenceSurface`](lib/src/surface.rs) trait implemented as a `SystemParam`.
- [`movement-intelligence-avian`](avian) — collider-backed surface over Avian `Fixed` geometry. Native product is [`AvianColliderPath`](avian/src/path.rs); interactions convert with `From`.
- [`movement-intelligence-richmond`](richmond) — composes the Avian surface with Les Halles storey / stairwell IR ([`CirculationStairwell`](richmond/src/circulation.rs)). Same-storey queries stay collider `MoveTo`s; a storey change prepends a tread polyline.

The brain writes [`MoveWish`](../player/src/body.rs). It does not lock onto other entities: a higher-order system writes the objective and inserts [`ReplanMovement`](lib/src/user.rs) when it wants a new plan.

Budget, vantage standoffs, and azimuths live on per-character [`MovementAbility`](lib/src/ability.rs) ([`Covering`](lib/src/ability.rs)). [`MovementIntelligenceLimits`](lib/src/surface.rs) is a system-wide max; each replan uses `character.clamp_to(limits)`. Avian `VantageOn` ranks hide/sightline cheaply, then walk-probes the best standpoints first.

Walk colliders for Richmond IR live in [`richmond-building-physics`](../richmond/building-physics): Fixed cuboids on floors, walls, and treads, separate from LOD Host volumes.

The brain writes [`MoveWish`](../player/src/body.rs). It does not lock onto other entities: a higher-order system writes the objective and inserts [`ReplanMovement`](lib/src/user.rs) when it wants a new plan.

Budget, vantage standoffs, and azimuths live on per-character [`MovementAbility`](lib/src/ability.rs) ([`Covering`](lib/src/ability.rs)). [`MovementIntelligenceLimits`](lib/src/surface.rs) is a system-wide max; each replan uses `character.clamp_to(limits)`. Avian `VantageOn` ranks hide/sightline cheaply, then walk-probes the best standpoints first.
