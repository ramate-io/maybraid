# Tether intelligence

Stay near, or on a ring around, a live entity. [`TetherIntelligenceUser`](src/user.rs)
is the installed brain; [`TetherMemory`](src/memory.rs) is the last observation
and survives uninstall. Higher-order systems install/uninstall the user and set
[`TetherIntelligenceUser::enabled`](src/user.rs); this crate owns the check /
replan loop while the user is present and enabled.

Close remaining work writes [`Reach`](../movement/lib/src/objective.rs) (tether)
or [`EdgeOf`](../movement/lib/src/objective.rs) (stalk). Far remaining work sets
a [`RoutingIntelligenceUser`](../routing/src/user.rs) destination. The plugin is
cadence-neutral; applications own the timer on [`TetherSystems::Write`](src/plugin.rs).
