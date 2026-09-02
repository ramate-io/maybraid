# Intelligence

Higher-order brains write objectives; lower-order crates field them.

## Movement

- [`movement-intelligence`](movement/lib) — [`MovementIntelligence`](movement/lib/src/user.rs) on a capsule. It fields [`MovementObjective`](movement/lib/src/objective.rs) and writes [`MoveWish`](../player/src/body.rs). It does not lock onto other entities.
- [`movement-intelligence-avian`](movement/avian) — collider-backed surface over Avian `Fixed` geometry.
- [`movement-intelligence-richmond`](movement/richmond) — composes the Avian surface with Les Halles storey / stairwell IR.

A higher-order system writes the objective and inserts [`ReplanMovement`](movement/lib/src/user.rs) when it wants a new plan. Budget and vantage *sampling* live on [`MovementAbility`](movement/lib/src/ability.rs). Hide / sightline *policy* belongs on the writer (firearm movement, etc.).

Walk colliders for Richmond IR live in [`richmond-building-physics`](../richmond/building-physics).

## Combat

- [`firearm-intelligence`](combat/firearm) — [`FirearmIntelligence`](combat/firearm/src/combat.rs) fields [`FirearmObjective`](combat/firearm/src/target.rs) (who to shoot). [`FirearmMovementIntelligence`](combat/firearm/src/movement.rs) fields [`FirearmMovementObjective`](combat/firearm/src/target.rs) and writes `MovementObjective` + `ReplanMovement`.
