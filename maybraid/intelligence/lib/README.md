# Movement intelligence

Install [`MovementIntelligence`](src/user.rs) on a capsule and register
[`MovementIntelligencePlugin`](src/plugin.rs) with a
[`MovementIntelligenceSurface`](src/surface.rs) `SystemParam`.

Writes [`MoveWish`](../../player/src/body.rs). Does not own the capsule or
follow other entities — higher-order systems write
[`MovementObjective`](src/objective.rs) and insert
[`ReplanMovement`](src/user.rs) to rebuild the plan.
