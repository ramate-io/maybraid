# Spotting Intelligence Avian

`spotting-intelligence-avian` provides the practical Avian backend for
[`spotting-intelligence`](../lib/README.md).

```rust
use bevy::prelude::*;
use spotting_intelligence_avian::SpottingAvianPlugin;

App::new().add_plugins(SpottingAvianPlugin);
```

On each `Update` in `SpottingSystems::Observe`, the plugin:

1. discovers subjects with an Avian sphere query restricted to
   `PhysicsInteractionLayer::Animated`;
2. skips directives already satisfied by fresh contacts;
3. merges discovery work with due contact refreshes;
4. ranks and caps candidates and visibility samples; and
5. tests samples against `PhysicsInteractionLayer::Fixed`, updating successful
   contacts and failure/retry state.

`clear_segment` is public so aiming and firing systems can use the same finite
segment obstruction rule.

`SpottingObserveLimits` separates discovery observers (`max_observers_per_tick`,
default 8) from additional retained-contact observers
(`max_retained_observers_per_tick`, default 8). Discovery merges its own due
contacts into the usual executor. After discovery, a private priority-ordered
service round grants bounded contact-only refreshes to additional observers;
no observer is probed twice in one tick. Each grant still uses the observer's
candidate and visibility-sample caps. Contact-only work cannot discover hints or
broadphase subjects. Budget deferral leaves observation timestamps and failure
state untouched; ordinary stale/invalid-contact cleanup runs independently.

The retained round persists across ticks and does not depend on shared
`IntelligenceLod.skips` resets. Despawned, no-longer-due, and discovery-serviced
entries are discarded when popped. A drained round is refilled on the next tick,
so a fixed priority prefix cannot re-enter ahead of already waiting observers.
This bounds service work, not contact age: with the world's 125 ms observation
cadence, a long round can still exceed the 200 ms firing freshness window.
Unknown-actor discovery fairness and application cadence remain separate policy.

Trace-level `spotting_discovery` and `spotting_retained_refresh` spans expose
observer grants and allocated visibility samples, collected in the existing
execution loops without an additional diagnostic scan. Enable just these coarse
phases with `RUST_LOG=info,maybraid.spotting.profile=trace` during recordings;
the spans use that dedicated target and allocate no per-ray spans.
