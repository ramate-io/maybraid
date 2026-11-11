# Spotting Intelligence Avian

`spotting-intelligence-avian` provides the practical Avian backend for
[`spotting-intelligence`](../lib/README.md).

```rust
use bevy::prelude::*;
use spotting_intelligence_avian::SpottingAvianPlugin;

App::new().add_plugins(SpottingAvianPlugin);
```

The plugin installs ordered `Prepare`, `Discover`, and `Refresh` subsets inside
`SpottingSystems::Observe`. Preparation expires stale and invalid contacts and
resets the per-Update service ledger. Discovery uses an Avian sphere query on
`PhysicsInteractionLayer::Animated`, skips satisfied directives, merges due
contacts, and ranks and caps visibility candidates. Retained refresh uses the
same executor with known contacts only. Both test samples against
`PhysicsInteractionLayer::Fixed`; only successful visibility updates sight age.
`clear_segment` exposes the same finite-segment obstruction rule to aiming.

The library imposes no timer. Applications can gate `SpottingSystems::Discover`
while retained refresh and preparation run each Update. Gating the parent
`Observe` set still gates all three children. `SpottingAvianPlugin` is the normal
entry point. Callers registering the exported systems manually must initialize
`IntelligencePriority`, `SpottingObserveLimits`, and `SpottingFrameState`, then
register `prepare_spotting` → `discover_spotting` → `refresh_spotting` in that order
each Update. The frame resource's service fields are private.

`SpottingObserveLimits` gives each phase independent observer and allocated-sample
caps. Defaults allow 8 discovery observers and 8 additional retained observers,
with 384 allocated samples per phase. An observer's own candidate/sample settings
and LOD scaling still apply, and each grant is clamped to the remaining phase
sample allowance. Actual ray count cannot exceed allocated samples. No elapsed
time, stalled frame, or disabled tick earns catch-up credit. Zero sample credit
leaves the waiting queue head and observation state untouched.

The world currently tries discovery of at most 8 observers every 125 ms and
retained refresh of at most 8 additional observers every Update, with 80 allocated
samples per phase. This bounds each Update to 160 samples on discovery frames
and 80 otherwise. Discovery and refresh share a frame ledger so an observer
cannot be probed by both in the same Update. Contact-only work never promotes
unknown hints or broadphase subjects to refresh targets. Budget deferral does
not advance success/attempt/failure timestamps or the sample cursor; ordinary
contact cleanup runs independently of both budgets.

The retained priority-ordered round persists across Updates independently of
shared `IntelligenceLod.skips` resets. Despawned, no-longer-due, and discovery-served
entries are discarded when popped. A drained round is refilled on the next
Update, so a fixed priority prefix cannot re-enter ahead of waiting observers.
Discovery fairness for unknown actors remains separate policy.

These bounds do not guarantee 200 ms firing freshness. At 30 FPS, 8 grants per
Update provide at most 240 observer grants per second; a large cohort and the
125 ms respot interval can exceed the firing window even at higher frame rates.
Failed LOS also legitimately leaves successful sight old. Compare attempt age
and failure state as well as success age when evaluating capacity. The world
trial needs runtime cost and contact-age measurements before its caps are final.

Trace-level `spotting_prepare`, `spotting_discovery`, and
`spotting_retained_refresh` spans expose phase timings. Discovery and refresh also
include entry-known due/queued counts and caps. Their child
`spotting_discovery_grants` and `spotting_retained_grants` spans report aggregate
observer grants and allocated samples after each existing execution loop. Counts
are set on span entry so `tracing-tracy` 0.12 captures them; no late field updates,
extra diagnostic scans, or per-ray spans are required. Enable only these coarse
zones with `RUST_LOG=info,maybraid.spotting.profile=trace` during recordings.
