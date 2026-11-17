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
allocator overhead, map/tree metadata, reserved component and nested storage,
source generation graphs, Arc-owned model internals, mesh morph data, renderer
copies/slabs, GPU resources and assets whose CPU copy has been released. It does
not explain all malloc allocations or predict RSS/physical footprint. Take
explicit snapshots at comparable settled phases and compare them with separate
OS measurements; do not add the figures to RSS or footprint.

The ECS component total sums live archetype entity counts × each component's
inline layout size, including sparse components. The largest 32 component types
include names and instance counts. This excludes nested Vec/Arc allocations,
reserved columns and allocator overhead; it cannot attribute a multi-gigabyte
heap to a component merely because that component contains a pointer. Table slots
are counted once per table, not per archetype. Inline component totals and asset
payload figures describe different selected scopes, not a complete heap total.

Nonblocking cache lock acquisition only avoids waiting for a current writer;
walking a successfully locked map can delay a writer. Every explicit census can
hitch. Capture at a memory measurement boundary outside CPU timing windows.
