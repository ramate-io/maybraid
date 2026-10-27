# Controllers core

Sole [`InputRouter`](src/router.rs) for source → actor / UI assignment, plus
token-scoped capture and suspension. This crate depends only on
[`maybraid-virtual-pad-controller`](../virtual-pad/README.md) and Bevy.

```text
InputSource ──► InputRouter assignment
            ├──► RouteTarget::Ui(UiTarget)      ──► menu consumers
            └──► RouteTarget::Actor(ActorTarget) ──► character / Tanz / skill-map
Capture / suspension tokens are owner-scoped. Releasing one owner cannot
release another. Assignment loss clears delivery.
```

Controllers read the router; they never rediscover a "current player". The
lowest-ID Player fallback is resolved once at the host assignment boundary
([`InputRouter::assign_lowest_id_actor`](src/router.rs)) while the source is
connected. Reconnect is [`InputRouter::connect`](src/router.rs). Lifecycle
events are [`RoutingEvent`](src/router.rs) messages; use `MessageReader`, not
`take_events`.

Schedule contract: generic [`ControllerPipeline`](src/schedule.rs) stages.
Domain ordering lives beside domain systems. Plugin install order is not the
contract. See [ARCHITECTURE.md](../ARCHITECTURE.md) for the role inventory
and temporary adapters.

## Scoped delivery

`InputSource::VirtualPad(Entity)` identifies an entity-owned source.
`InputSource::local_pad()` is a transitional host alias for `PrimaryVirtualPad`;
new routed envelopes always preserve the canonical entity identity. The router
keeps assignment, captures, suspension and binding generations per source. Assigning
an already-controlled actor transfers ownership and cancels the previous source.
Combine intentional keyboard/mouse/gamepad input inside one virtual controller.

`ActorPadEvent { entity, input: Routed<ActorPadSnapshot> }` and
`UiPadEvent { entity, input: Routed<VirtualPadEvent> }` are targeted entity events.
The actor snapshot includes the rich source frame, capture/back capabilities and
suspension status. Shell input can reach its assigned actor while gameplay is
captured or frozen; strict gameplay routes cannot. Any capture kind can deliver
UI input to its explicit `ScopeOwner::Ui(UiTarget)` entity. Core does not interpret
map, picker, menu, console or character behavior.

Both destinations are bound before dispatch. `PadDeliverySystems` runs inside
`ControllerPipeline::Bind` after `CaptureResolveSystems`: source/lifecycle
resolution, immutable destination binding, UI dispatch, an explicit deferred flush,
then actor dispatch. Receivers reset ephemeral actor state before that public set.
A UI handler calls `input.consume_frame(&mut commands)` when it wins a Select,
Back or context action. This suppresses competing actor/shell delivery and nested
actor actions for that source and sequence; other sources and the UI's own queued
actions remain live.

Derive actions with `input.clone().map(...)`, then call
`input.trigger(&mut commands, |input| TargetedAction { entity, input })`.
The central gate validates the original binding generation, target existence,
consumed frame and event target immediately before deferred execution. Captures,
focus changes, reassignment, disconnect and suspension retire old generations;
returning to the same actor does not revive queued input. `input.binding()` is
an opaque equality token for receiver-owned gesture/repeat provenance. Consumers
do not poll the router or re-resolve an actor.

`ActorRoutingEvent` addresses lifecycle effects and an `ActorRouteStatus` snapshot
to the affected actor. `InputRouting` publishes this targeted status when mutation
finishes, including mutations later in the update, alongside compatibility
`RoutingEvent` messages. Production routing mutations use `InputRouting`; direct
`ResMut<InputRouter>` remains a transitional host/test API with scheduled publishing.
Legacy pad resources and unscoped message APIs are compatibility adapters for later
consumer commits in the migration series ([#1502](https://github.com/ramate-io/maybraid/issues/1502)).

Core records each source's routing generation immediately after hardware produces
its frame in `PreUpdate`. If capture resolution or actor reassignment retires that
generation before `Update` delivery, that physical frame is discarded; its edges
cannot be replayed on the new owner. The actor snapshot also freezes that source's
`trigger_press_threshold` for character interpretation.

Actor delivery owns a `PadRearm` per source. Reassignment, disconnect, capture and
capture release cancel held controls until each button/key releases and each
stick/trigger returns to neutral. The release edge of a canceled B hold is also
removed, so it cannot become a stance tap on another actor. Raw UI snapshots stay
unchanged. Shell Start/Select and their keyboard capabilities remain available;
suspension/resumption freezes ownership without adding a release requirement.
`set_source_paused` resumes only that source's pause token.

Host capture systems in `PreUpdate` order before `PadPolicySystems`; policy then
runs before virtual-pad production. Core also releases all capture layers whose
actual owner entities have despawned, including layers below the current top.
Legacy process-wide placeholder owners remain compatibility exceptions.
