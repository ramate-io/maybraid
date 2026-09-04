# Tether intelligence

Stay near, or within a stalking annulus around, a live entity.
[`TetherIntelligenceUser`](src/user.rs) is the installed brain;
[`TetherMemory`](src/memory.rs) is the last observation and survives uninstall.
Higher-order systems install/uninstall the user and set
[`TetherIntelligenceUser::enabled`](src/user.rs); this crate owns the check /
replan loop while the user is present and enabled. A retracted grant does not
write [`TetherAction::Hold`](src/user.rs): Hold pins position, so disable must
leave movement for flee or firearm.

Close remaining work writes [`Reach`](../movement/lib/src/objective.rs) (tether)
or [`EdgeOf`](../movement/lib/src/objective.rs) at the nearest inner/outer
stalking boundary. Far remaining work routes to that boundary. The plugin is
cadence-neutral; applications own the timer on
[`TetherSystems::Write`](src/plugin.rs).
