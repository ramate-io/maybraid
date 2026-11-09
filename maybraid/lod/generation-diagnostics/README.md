# Generation diagnostics

This optional crate records HCSG generation work for Tracy. It installs no
subscriber and is compiled into the game only with the default-off
`generation-tracing` feature. The existing Bevy logging/Tracy subscriber handles
its dedicated target.

```sh
cargo build --release -p maybraid --features generation-tracing,bevy/trace_tracy
RUST_LOG=info,maybraid.generation.profile=trace ...
```

Leave `maybraid.generation.profile` filtered off for renderer performance
comparisons. Enabled diagnostics add allocations, clocks, spans, and counter
mutex operations, including on cache lookups. A separate generation diagnosis
can explain behavior; its timings are not an uninstrumented renderer baseline.
With the feature off, instrumentation fields/calls are compiled out. With the
feature on but target off, operations, clocks, and correlation allocation are
not started.

`hcsg.operation` scopes actual synchronous/worker context work and application
or maintenance boundaries. Contexts retain an accumulator and unentered span,
not a thread-local entered guard. Lexical work guards establish attribution;
contexts can be dropped in a different order from creation. Completion waits
for the last active scope. Worker requests reuse subscription ID, epoch, and
per-subscription quantum number. `hcsg.enqueue`, `hcsg.cancel`, and
`hcsg.quantum.complete` describe lifecycle. The queue-wait value is initial or
resumed enqueue-to-pick delay, separate from quantum execution. The existing
scheduler, cancellation policy, cache, and publication order are unchanged.
Lifecycle messages are emitted after the demand guard is released; a worker can
pick the job before its enqueue message is emitted. The enqueue timestamp is
captured at queue eligibility, and `enqueue_age_ns` exposes the emission delay.
Use identity/epoch/quantum joins rather than message ordering for causality.
Publication detail is likewise emitted after releasing the store write guard.

Every operation emits `hcsg.operation.complete` as a structured tracing event
with final values already present. The tracing-tracy 0.12 adapter formats it as
a message for the Tracy 0.14.1 collector. Export with `tracy-csvexport -m`; do not
use late span updates or empty count zones to recover accounting. The headless
`lod` example `generation_trace_fixture` supports a small export check; its
actual verification results belong in the capture companion, not this source
contract.

Default operation spans contain only stable origin/family labels; operation IDs,
subscription IDs, epochs, sessions, and timings are completion-message fields,
not zone-name fields. Default phase spans contain stable phase/family labels, with at most 64 coarse
phase spans per operation. Full scheme names and nested dependency spans require
explicit opt-in:

```sh
MAYBRAID_GENERATION_TRACE_DEPTH=3 \
MAYBRAID_GENERATION_TRACE_TYPE=Richmond \
MAYBRAID_GENERATION_TRACE_DETAILS=32 ...
```

Depth is capped at 16 and detail count at 256 per operation. Suppressed detail
and coarse spans are counted in the final summary. The family label is a small
classification of scheme names, not a second generation registry. Unknown
families remain `other`.

Interpretation:

- Cache hit/miss counts describe `GenerationContext` value lookups; they are not
  all possible storage/index reads. `builds` includes recursive builds, and
  `absent` means `build_with_id` returned no value.
- Registry/store acquisition and held durations are separate. They cover the
  existing RwLocks, including successful try reads; `busy` records unsuccessful
  try reads observed inside an operation. Demand mutex/Condvar idle waiting is
  not included in those lock counters.
- Discovery can itself generate dependencies. Build, discovery, publish, lock,
  and active-operation durations are inclusive and overlap. Do not add them to
  estimate exclusive CPU. Context lifetime also includes time between calls;
  active duration covers lexical work scopes.
- Publication counts track successful storage publication. First/last versions
  are endpoints, not a complete contiguous version list under concurrency.
  Cached available IDs in a worker quantum are not new builds or publications.
  Publication spans bracket the storage boundary at depth zero; they do not
  represent recursive dependency depth.
- Presentation reads and retirement counts describe queued host commands.
  `presentation.apply` is the actual deferred HcsgNode insertion, correlated by
  its storage version. It does not claim to time full scene fulfillment, mesh
  generation/upload, or arbitrary ApplyDeferred work.
- Maintenance covers the existing sweep, session-clear, and retired-host
  despawn boundaries without added discovery scans. Removal counts use existing
  store lengths under the same lock; they work in release, unlike the older
  debug-only eviction-identity tracking.
- `epoch_known=false` means no HCSG worker epoch was attached; the separate
  application-session field is presentation's local session, not that epoch.
  Opt-in `hcsg.publish` detail events carry exact node/version/operation joins;
  their shared detail cap can suppress entries. Default summary version endpoints
  must not be expanded into an invented contiguous list.
- Counter/event completeness is limited to enabled operations and the scope
  exercised by the verified capture fixture.

To verify a fixture export with the checked-in parser:

```sh
tracy-csvexport -m fixture.tracy > messages.csv
tracy-csvexport -f hcsg.operation fixture.tracy > operation-zones.csv
python3 maybraid/lod/generation-diagnostics/tools/verify-fixture-messages.py messages.csv --zones operation-zones.csv
# For the depth=8, details=2 fixture, add --detail-limit 2.
```

The verifier deliberately covers synchronous miss/hit/publication accounting,
non-LIFO context ownership, queued replacement cancellation and one worker
quantum, plus all 1,000 cache-hit completion events. With the zone export, it
also verifies only origin/family fields appear in operation names and all 1,000
cache-hit entries share one aggregate zone. It does not establish trace
coverage of arbitrary scenes, command application, or every maintenance path.
