# Intelligence

Movement intelligence crates for Maybraid.

- [`movement-intelligence`](lib) — [`MovementIntelligence`](lib/src/user.rs) on a capsule, objectives, and a [`MovementIntelligenceSurface`](lib/src/surface.rs) trait implemented as a `SystemParam`.
- [`movement-intelligence-avian`](avian) — collider-backed surface over Avian `Fixed` geometry. Native product is [`AvianColliderPath`](avian/src/path.rs); interactions convert with `From`.

The brain writes [`MoveWish`](../player/src/body.rs). It does not lock onto other entities: a higher-order system writes the objective and inserts [`ReplanMovement`](lib/src/user.rs) when it wants a new plan.

Budget and vantage standoffs live on per-character [`MovementAbility`](lib/src/ability.rs) ([`Covering`](lib/src/ability.rs)). [`MovementIntelligenceLimits`](lib/src/surface.rs) is a system-wide max; each replan uses `character.clamp_to(limits)`.
