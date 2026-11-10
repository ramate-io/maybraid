# Native renderer preparation diagnostics

This standalone diagnostic core has no Bevy dependency. Its `diagnostics` feature
is default-off, and the unpatched game does not depend on it. Ordinary workspace
and all-features builds remain valid without a patched renderer. The explicit
source override below instruments the exact native Bevy 0.20.1 writer; it does
not change render entities, batching decisions, buffer writes or draw commands.

## Opt-in native dirty-upload prototype

The additional [native input patch](../../../profiling/patches/bevy-render-0.20.1-native-dirty-uploads.patch)
wraps only the three retained, CPU-authored, shader-read-only binned inputs. It
preserves ordinary per-frame lookup, jobs, global metadata, fan reservation and
GPU dispatch. Normal registry rendering remains unchanged. The optimization is
compiled only through the explicit native source override; its runtime default
is **force**, which always requests the original full nonempty writes.

Prepare the clean timing flavor in a fresh external directory:

```sh
python3 profiling/prepare-render-prep-diagnostics.py --native-dirty-uploads \
  --output /private/tmp/native-binned-clean-r1
cargo build -p maybraid --release --features bevy/trace_tracy \
  --config /private/tmp/native-binned-clean-r1/render-prep-cargo.toml
```

The same immutable binary supports process-start
`MAYBRAID_NATIVE_BINNED_UPLOADS=force` (A) and `=dirty` (B). The mode is frozen
once per process and logged as `native_binned_dirty_uploads mode=...`.
Missing/unrecognized values retain force behavior. Clean native preparation
activates `native-binned-dirty-uploads` only; detailed counters, byte snapshots
and per-buffer diagnostic spans are compiled out. Use matched native wgpu log
filters for A/B; do not enable native INFO scopes in only one timing arm.

For a separate correctness build, add `--diagnostics` to preparation. This also
compiles the sampled census and an independent last-submitted byte snapshot for
each input. Every would-skip checks actual bytes, even between sampled census
frames. Any `native_binned_dirty_mismatch` error rejects the run, although the
renderer forces an upload to preserve output. No payload bytes are logged. The
checks have no unbounded per-skip spans; existing bounded diagnostic scopes and
counters remain available. Set `MAYBRAID_RENDER_PREP_DIAGNOSTICS=1` to emit the
sampled census; byte validation is active throughout the diagnostic build.

`logical_nonempty_requests` and `logical_bytes` describe the current requests;
`queue_calls`/`upload_bytes` and `skipped_uploads`/`skipped_bytes` must conserve
those totals for each of the three inputs. Fan is scratch and excluded.
`shadow_checks` and `shadow_mismatches` report independent skip validation;
`queued_equal_uploads`/`queued_equal_bytes` report queued payloads equal to the
previous submission. Equal payloads can still require upload after destination
replacement, so that last metric alone is not proof of a removable write.

Mutation invalidation occurs before returning mutable access from native
push/set/values_mut/swap_remove. It is conservative: a same-value set or an unused
mutable borrow can request another upload. Generation wrap invalidates queued
state. The dirty path processes reserve/labels before comparing the actual
BufferId, and recreates storage while preserving usage, label and CPU values if
the actual device changes. Empty/skipped/ineligible paths never acknowledge a
new submission. State belongs to each phase/view/batch-set/input; unchanged
counts or a global cache do not establish valid contents.

Preparation also creates `wrapper-tests/Cargo.toml`, a focused harness including
the **actual patched wrapper file** against pinned public Bevy APIs. Its tests
exercise native mutation methods and independent input state without a GPU:

```sh
cargo test --release --manifest-path /private/tmp/native-binned-clean-r1/wrapper-tests/Cargo.toml
cargo test --release --manifest-path /private/tmp/native-binned-clean-r1/wrapper-tests/Cargo.toml \
  --features render-prep-diagnostics
```

Use the same coordinated Nix/native compiler setup as the game. Source/build
validation and a byte-safe correctness run must precede performance claims.

## Prepare a diagnostic source override

From an isolated implementation worktree with the normal registry `Cargo.lock`:

```sh
python3 profiling/prepare-render-prep-diagnostics.py \
  --output /private/tmp/maybraid-render-prep-source-r1
```

The helper verifies the published `bevy_render-0.20.1.crate` against its pinned
Cargo.lock SHA256, rejects unsafe archive members, extracts only that crate into
a new external directory, and applies the small checked-in
[writer patch](../../../profiling/patches/bevy-render-0.20.1-render-prep.patch).
It never edits Cargo registry sources or an existing output directory. Pass
`--archive /absolute/path/bevy_render-0.20.1.crate` if the Cargo cache is ambiguous.
No network request or build runs in the helper.

The generated `render-prep-cargo.toml` overrides only `bevy_render`. Its patched
manifest has an optional path dependency on this core and a default-off
`render-prep-diagnostics` feature. In this external copy only, Bevy's `trace`
feature enables the diagnostic feature. Both the explicit `--config` and the
normal `bevy/trace_tracy` build feature are therefore required:

```sh
cargo build -p maybraid --release --features bevy/trace_tracy \
  --config /private/tmp/maybraid-render-prep-source-r1/render-prep-cargo.toml
```

Use the project's Nix environment. Preserve its linker search flags and follow
local build coordination before a heavy build. A path override changes the
resolved lock source. The first diagnostic resolution must allow the isolated
checkout's `Cargo.lock` to update. Save/hash it as `resolved-Cargo.lock` next to
the generated manifest; then use `--locked` for later diagnostic commands.
The helper saves `input-Cargo.lock` and records its SHA256. Restore the normal
registry lock before committing; never commit an external path-source lock.
Record the source commit/tree, executable hash, generated config/manifest,
archive and patch hashes, effective resolved lock hash, and build features with
each capture. Changing core source after preparation requires fresh preparation
or an explicitly refreshed manifest; the core is a live path dependency.

## Capture ordinary gameplay

Use a controlled **capture-only** runner with a frozen run manifest. Pin the
absolute path and SHA256 of the verified, immutable patched executable, its
source/build manifest and explicit patch configuration. The runner must launch
that exact executable; a checkout's `target/release/maybraid` is not an acceptable
fallback. Also pin the absolute collector executable and its hash. This dependency
graph requires Tracy **0.14.1 / protocol 82**; stock Nix Tracy 0.13.1 is incompatible.

Freeze the script, viewport, VSync, actual camera pose and motion controls before
comparison. Set the run manifest's environment to
`MAYBRAID_RENDER_PREP_DIAGNOSTICS=1`,
`MAYBRAID_RENDER_PREP_SAMPLE_FRAMES=120` and
`RUST_LOG=info,wgpu=error,maybraid.poi.profile=off,wgpu_core::device::queue=info,wgpu_core::resource=info`.
The current natural-area runner supplies these values through its frozen
configuration rather than inheriting arbitrary shell environment overrides.
Do not add a synthetic scene to inflate a denominator.

The render-prep capture procedure verifies source, binary and tool hashes,
requires exclusive ownership of the game and collector, establishes warmup and
foreground/window validity, then attaches the collector before the premeasurement
screenshot gate. Its generated frozen launcher has this invocation shape:

```sh
bash /absolute/path/to/frozen-native-prep/run.sh \
  --phase cpu --output /absolute/new/capture-directory
```

Prepare and review that launcher/configuration for the specific run first; the
placeholder above is not a shipped repository command. Use a fresh output path
and finite wall-clock, game-log and capture-size limits. The current procedure
uses 750 seconds, 2 GiB for the game log and 8 GiB for the on-disk capture,
terminating only its owned processes on failure. The file-size limit cannot see
collector data still buffered in memory. Verify deliberate game completion,
collector completion, window/camera evidence and capture provenance afterward.

Do **not** use the legacy `profiling/tracy-flight.sh` for this workflow. It defaults
to a checkout binary and automatically runs aggregate/message exports and an
unfiltered `tracy-csvexport -u` stream. Merely setting `MAYBRAID_BIN` fixes neither
its implicit exports nor their lack of finite budgets. The controlled collector
produces the `.tracy` and run evidence only. Export separately after capture under
an explicit offline export scope: exact requested zones/messages and measurement
window, with finite time, output-byte and record limits. Never implicitly export
all zones or an unrestricted unwrap stream.

The narrow wgpu overrides expose existing INFO-level `Queue::write_buffer`,
`StagingBuffer::new` and copy profiling scopes in wgpu 30.0.1. Bevy's default
filter includes `wgpu=error`; compiling `profiling/profile-with-tracing` alone
does not make those scopes visible. Verify their presence in an actual capture.
They measure synchronous CPU work. Tracy has no Metal GPU pass timings here.

## Completed census and timing scopes

`render_prep_census` is one completed numeric message per sampled extracted view
and native binned phase. Its frame is the existing extracted Bevy `FrameCount`
(`bevy_render::globals::extract_frame_count`), not a private observer counter.
The startup interval defaults to 120 and is clamped to at least 1; frame numbers
divisible by the interval are sampled. Runtime switch off or skipped frames do
not scan retained maps or snapshot buffers. Feature-off removes instrumentation.

The patch runs inside `write_binned_instance_buffers`, after native phase
batching and at the actual writer boundary. It accesses the native retained
phase and native vectors directly, avoiding stale post-writer job descriptors.
It never scans the gameplay world or logs per entity. The same generic hook
covers Opaque3d first and all other native binned phases that run this writer.

| Fields | Meaning |
|---|---|
| `frame`, `phase`, `view`, `mode` | Extracted app frame, native phase type, retained view identity and actual writer mode, including phase absence or skipped mode. |
| `candidate_sets`, `bins`, `instances` | Current multidrawable retained sets, native bin count and instance-entry vector lengths before eligibility. |
| `batchable_*`, `unbatchable_*`, `non_mesh_*` | Retained bin counts and sums of per-bin entity-map lengths. |
| `lookup_input_sets`, `lookup_entries` | Native representative lookup input and resulting metadata lookup entries. |
| `eligible_sets`, `skipped_mode_sets`, `skipped_no_representative`, `skipped_no_metadata` | Actual writer branch outcomes before native buffer writes. Candidate sets equal eligible plus the three pre-write skip counts. |
| `skipped_missing_buffers`, `skipped_index_overflow`, `indexed_jobs`, `non_indexed_jobs` | Actual post-write skip or job-enqueue outcomes for this invocation. |
| `buffers[0..4]` | Instance entries, bin metadata, bin-index lookup, fan scratch, in that order. |
| Buffer `entries`, `logical_bytes` | Sum of actual CPU/native vector lengths and length × native struct size for eligible sets. Lookup lengths include holes. Fan logical bytes are scratch extent. |
| Buffer `queue_calls`, `upload_bytes` | Explicit CPU queue write requests whose Rust API calls returned; dirty-mode skipped calls are counted separately. Fan is always zero. These do not assert GPU execution or successful transfer completion. |
| Buffer `capacity_before`, `capacity_after`, `destination_bytes_before`, `destination_bytes_after`, `capacity_growths` | Observed native capacity and public destination byte sizes immediately around each write/reserve, with exact growth counts. |
| Buffer `observed_handle_changes` | Actual Bevy BufferId changed/created across the call. Kept distinct from capacity growth and driver allocator events. |

Three nonempty native RawBufferVecs each request one full logical upload per
eligible batch set, regardless of retained CPU contents. Fan uses UninitBufferVec
and only materializes scratch storage. The input arrays are shader read-only;
other GPU-written output/preprocess/indirect buffers are separate. Do not infer
a safe dirty-skip optimization from unchanged counters alone. A mesh can appear
in several views/phases; sums are not unique global entity counts.

Stable scopes are `render_prep_writer` (entire writer), `render_prep_lookup`
(per-view metadata map), `render_prep_upload_jobs` (per-view writes plus job
assembly), and `render_prep_buffer` (individual typed reserve/write). Detailed
buffer spans are a prefix capped at 64 **per phase per sampled frame**. Counters
still include every eligible write for each emitted view after that cap.
`render_prep_detail_truncated` reports omitted detail spans. The view census is
capped at 32 extracted views per phase per sampled frame, with
`render_prep_truncated` reporting omitted views. View inclusion and detail order
are not stable; do not extrapolate prefix timings to the whole writer. No fields
are recorded late onto spans, and the complete values are formatted into each
summary message.

Writer, lookup, upload/job, buffer and native wgpu scopes are nested inclusive
CPU times. Do not sum them as independent cost. Only same-thread direct-child
subtraction can estimate self time; descheduling/locks/driver work remain inside
those elapsed intervals. The writer residual also includes the sampled census
prescan and completed-message formatting; it is not pure native bookkeeping.
Initial diagnosis should use the existing wgpu scopes
before proposing any deeper instrumentation or renderer optimization.
