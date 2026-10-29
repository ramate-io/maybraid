# Grenade user

Held grenade, cook / windup / release, and owned recharge clocks.

Throw realization reads addressed
[`EquippedItemUse`](../equipment/src/dispatch.rs) for the
grenade [`EquippedItemHandler`](../equipment/src/dispatch.rs).
A missing use is not a release. [`apply_throw_routing`](src/throw/lifecycle.rs)
observes actor-addressed routing events. Capture keeps a live
fuse ticking and only cancels a leftover use latch; suspension freezes both
fuse and windup in [`advance_throw`](src/throw/execute.rs). Disconnect /
reassign / removal drop through
[`GrenadeThrow::apply_routing`](src/throw/mod.rs).
[`GrenadeThrow::take_live_cook`](src/throw/mod.rs) is the single swap / death /
cancel drop owner. [`spawn_held_grenade`](src/hold.rs) installs the grenade
handler. Execution (cook / windup, routing, recharge, equipment realize) lives
in [`runtime`](src/runtime.rs) via [`register_grenade_user_runtime`](src/runtime.rs).
HUD / pose / feel live in [`presentation`](src/presentation.rs). Throw publishes
[`SpawnThrownGrenade`](../grenades/src/runtime.rs); `maybraid-spawns-grenades`
realizes its simulation body in [`realize.rs`](../../spawns/grenades/src/realize.rs),
and presentation attaches visuals afterward.

See [ARCHITECTURE.md](../../controllers/ARCHITECTURE.md) for the D1 / D3
contract.
