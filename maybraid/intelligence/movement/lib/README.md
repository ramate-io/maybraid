# Movement intelligence

Install [`MovementIntelligence`](src/user.rs) on a capsule and register
[`MovementIntelligencePlugin`](src/plugin.rs) with a
[`MovementIntelligenceSurface`](src/surface.rs) `SystemParam`.

Writes [`MoveWish`](../../player/src/body.rs) as an XZ heading toward the next
waypoint. The capsule motor projects that onto the current walkable plane
while grounded; this crate does not own physics. Higher-order systems write
[`MovementObjective`](src/objective.rs) and insert
[`ReplanMovement`](src/user.rs) to rebuild the plan.

Per-character [`MovementAbility`](src/ability.rs) owns covering (budget, vantage standoffs).
[`MovementIntelligenceLimits`](src/surface.rs) caps that budget for the app.
