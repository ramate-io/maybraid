# Explicit CPU ownership census

Build the game with `--features memory-census`, enable `--agent-diagnostics`,
then send the agent console command `diagnostics memory`. For example, a script
command is `{"cmd":"console","line":"diagnostics memory"}` (use the normal
agent script envelope). The `memory_census` JSON event carries `command_id` and
its `census`. The command acknowledgment says the capture was queued; the census
is taken during command application and may stall that frame. Do not use its
frame as a timing observation. Diagnostics must be on.

The crate installs no systems and does no background scanning or logging. The
feature is off by default. It enables Bevy's `debug` feature so release builds
retain component names; this is another reason to keep plain-release footprint
controls separate. Component IDs are local to the captured World, not cross-run
identities. ECS counts include empty archetypes retained by Bevy;
table capacity counts entity slots, not component bytes or archetype allocation
capacity. Live entity counts include Bevy's resource and infrastructure entities,
not just gameplay actors or rendered objects. Mesh attribute/index Vecs and Image
data Vecs report initialized logical
bytes separately from reserved element capacity. Capacity already includes the
logical bytes: do not add them. Assets are visited once by AssetId. Shared live
Mesh3d handles count as references but their payload is counted once.

The selected Durham terrain/water mailbox snapshots report entry counts,
HashMap entry capacities and unique mesh IDs. Overlay/EnforcedCaches Arc aliases
are not separate maps. Busy or poisoned maps are explicitly unavailable; they
are not reported as empty. A read lock is held for an explicit O(entries) walk.
The mesh payload totals per map overlap the global asset total and can overlap
other owners. Do not add them. Mesh3d absence does not establish cache-only
ownership: deferred commands, queues, scene caches and other handles can retain
assets. No Arc strong-count inference or cache eviction is performed.

This is a selected main-world CPU payload census, not heap accounting. It omits
allocator overhead, map/tree metadata and nested storage,
source generation graphs, Arc-owned model internals, mesh morph data, renderer
copies/slabs, GPU resources and assets whose CPU copy has been released. It does
not explain all malloc allocations or predict RSS/physical footprint. Take
explicit snapshots at comparable settled phases and compare them with separate
OS measurements; do not add the figures to RSS or footprint.

The ECS component total sums live archetype entity counts × each component's
inline layout size, including sparse components. The largest 32 component types
include names and instance counts. This excludes nested Vec/Arc allocations,
sparse-set reservation and allocator overhead; it cannot attribute a multi-gigabyte
heap to a component merely because that component contains a pointer. Table slots
are counted once per table, not per archetype. Inline component totals and asset
payload figures describe different selected scopes, not a complete heap total.

Nonblocking cache lock acquisition only avoids waiting for a current writer;
walking a successfully locked map can delay a writer. Every explicit census can
hitch. Capture at a memory measurement boundary outside CPU timing windows.

## Gameplay ownership snapshots (schema 2)

The same explicit command additionally records dense-table initialized and
reserved inline data, added/changed ticks, and entity IDs. Empty, partial and full
partitions are mutually exclusive; a zero-row table is empty even at capacity
zero. Top 16 tables list up to 16 component layouts each; top 32 component totals
are bounded independently. Sparse archetypes sharing a dense table are counted
once. These totals overlap live inline component bytes above. They exclude nested
heaps, archetype/column metadata, sparse-set reservation, changed-by tracking and
allocator overhead. Reservation is not resident or reclaimable physical memory.

The mesh enabled subset means local `Disabled` absence. The ViewVisibility
subset means main-world visibility in any view, which can be stale or include a
shadow view. Neither establishes current camera submission, GPU residency or
exclusive ownership. Each subset counts shared AssetIds once and overlaps the
all-assets and all-Mesh3d totals; never sum these payloads.

The world adapter also reads existing LOD markers (including Disabled roots),
visibility-queue job/frontier counts, and cached generation counts. Frontier
entries are not unique entities or all remaining descendants. Generation reads
use `HcsgStorage::try_store_sizes()` only: a busy registry or typed store withholds
the whole type-count sample. The largest 32 store counts are emitted with total
stores/entries and omission count. Eviction-rebuild atomics are cumulative since
that storage was created, not allocation counts. Worker demand/backlog and graph
heap bytes have no aggregate nonblocking readout here and remain unknown.

Retired successor observations reuse [#1631](https://github.com/ramate-io/maybraid/pull/1631)
provenance only when `generation-tracing` and `MAYBRAID_RETIRED_HOST_PROFILE=1`
were enabled before retirement. Otherwise they remain unknown. A missing entity,
a present host without streamed content, and a content-streamed host are
separate states. Content-streamed does not establish desired LOD, visibility,
physics readiness or GPU readiness. Counts overlap other marker counts and do
not include descendants or bytes retained by each root.

For the next active-world measurement, take five explicit snapshots: initial
settled origin; first far arrival; far after 40 seconds quiet; actual origin after
return and 40 seconds quiet; same far checkpoint after repeating and settling.
Keep the same radius, camera, normal player health/support, and process identity.
Record separate five-second footprint/RSS samples continuously. The `maybraid.memory.census` / `memory_census_observer` Tracy span covers capture,
JSON conversion, emission and temporary-value cleanup for the command. Bind its
command ID/frame to the OS samples; observer-window peaks are distinct from
quiet gameplay peaks. Snapshot work
can hitch and delay generation writers after successful try-lock acquisition;
exclude census frames and their immediate neighborhood from CPU timing. Keep
Home, map observers and death outside this gameplay comparison. Missing quiet
coverage, failed return or busy owner data must remain explicit limitations.

Validation of this extension is pending. Focused commands for the normal Nix
shell (only with the machine compiler lease) are:

```shell
cargo test --release --locked -p maybraid-memory-census
cargo test --release --locked -p lod --features generation-tracing diagnostic_backlog_tests
cargo test --release --locked -p lod --features generation-tracing successor_readout_tests
```

```shell
cargo test --release --locked -p maybraid-world --features memory-census streaming_snapshot_includes_disabled_retired_roots_and_preserves_unknowns
```
Existing storage diagnostics fixtures cover registry and typed-store contention.
No native ownership snapshot or comparison is claimed by this source checkpoint.
