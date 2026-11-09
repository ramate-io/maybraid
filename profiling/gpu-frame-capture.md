# Metal render-update capture

Build the game with the **default-off** `gpu-frame-capture` feature to inspect a
warm GPU workload in Xcode. This dedicated diagnostic crate is wired only by the
game executable. Normal builds have no capture plugin, polling systems, or direct
Objective-C capture dependencies. Enabling the feature without the two runtime
paths also installs no polling systems.

```sh
cargo build -p maybraid --release --features gpu-frame-capture
```

Use an immutable copy of the resulting executable and a JSON manifest containing
`source_head` (full source commit), `binary_sha256`, `build_command`, and `features`
(including `gpu-frame-capture`). If building uncommitted changes, additionally
record their diff hash and preserve the source diff beside the manifest. Do not
reuse stale local Cargo package artifacts when making an immutable binary after
switching worktrees: clean the changed package closure before building.

```sh
python3 profiling/gpu-frame-capture.py \
  --binary /absolute/path/to/immutable-maybraid \
  --manifest /absolute/path/to/binary.manifest.json \
  --assets /absolute/path/to/maybraid/assets \
  --output-parent /absolute/path/to/profiling/captures/gpu-frame
```

The harness creates a new private run directory, verifies binary identity,
requires an otherwise idle profiling/build host, and launches a native 1920×1080
fullscreen game. It waits for surface readiness, 30 seconds of warmup, a completed
native screenshot, the `gpu-frame-ready` scenario mark, and foreground/window
readback before creating the capture trigger. It preserves the script, manifest,
environment, PID, foreground observations, screenshot, sidecar, and game log.
Readiness/capture waits and subprocess shutdown are bounded. The harness owns a
`caffeinate` assertion tied to its game PID, waits for the scripted clean game exit
after capture, and cleans only its own game/assertion processes on failure.

Foreground verification uses a small Swift helper compiled once before launch.
`NSRunningApplication` requests activation and reports active state; `NSWorkspace`
reports the frontmost PID; `CGWindowListCopyWindowInfo` supplies only the target
PID's on-screen layer-zero window bounds. The gate verifies target ownership,
active/frontmost identity, window dimensions, and the in-engine PNG. It does not
use Accessibility or take screenshots of unrelated windows. Helper build output,
return codes, stdout/stderr, source/binary hashes and the harness source commit
are preserved separately from the immutable engine manifest.

The first real ready-game attempt exposed an Accessibility denial when System
Events read `size of front window` (`osascript is not allowed assistive access`).
That attempt never wrote a trigger and is invalid; its successful screenshot is
not capture proof. The metadata helper replaces that permission-dependent read.

Use `--script path.jsonl` to supply a different stable scene. Include a
`gpu-frame-ready` scenario start **after** settled assets/pipelines and a screenshot
at `@RUN_DIR@/ready.png`, then idle long enough to receive the trigger. The default
view is the game's initial camera at `--start-at=976.6,1819.9`; scripts for a chosen
view must set it explicitly and preserve that script as evidence. Compare the
ready screenshot with the intended view before interpreting a trace.

## Capture protocol

For a custom harness, set `MTL_CAPTURE_ENABLED=1` before launch and provide both
`MAYBRAID_GPU_CAPTURE_OUTPUT` and `MAYBRAID_GPU_CAPTURE_TRIGGER`. They must be
distinct absolute paths directly inside the same canonical private directory
(mode 0700). Output must end in `.gputrace`. Existing output, trigger, claimed
trigger, or status paths are refused. Poll `capture-status.json` for `armed`, then
create the trigger once with its `request_id` as the entire file content. Stale or
foreign request IDs fail without starting capture. At the trigger, the plugin also
records `pipeline_pending_count` from `PipelineCache::waiting_pipelines()` and
`pipeline_failed_count` from cached `Err` states. Any nonzero count or missing cache
fails before claiming or starting capture. This refusal leaves the trigger as a
stale path, marks the run failed, and removes its polling state; it never retries.
The waiting set covers queued/creating entries and retryable shader errors after
Bevy has processed its queue; terminal errors can leave that set and are covered
by the separate failed count. The public iterator does not expose the private
new-pipeline queue before processing, so zero is a conservative readiness check
for the previously processed cache, not a guarantee against new work this update.
Warmup and the ready screenshot remain required scene/asset evidence. The trigger is atomically
claimed with a no-replace hard link and removed; no second capture is permitted.

The render-world exclusive begin system runs before `ExtractCommands` and
`PrepareAssets`. At the trigger it reacquires the active Bevy render device,
opens a scoped wgpu Metal HAL guard, and retains the native device while guarded.
The guard drops before rendering. `MTLCaptureDescriptor` selects
`GPUTraceDocument` and an explicit output URL. The exclusive end system runs after
`Render` and before `Cleanup`; Bevy's render system has submitted readback work and
presented the drawable by then. The sidecar records request/start/stop render
update numbers, PID, backend, device identity, output, and errors. It retains both
native and wgpu device identity for the owned session and rejects device recovery.
It never replaces wgpu's device-lost callbacks.

Only a successful start creates an owned capture session. That session stops once
after rendering, with an orderly-teardown fallback. Start failures never stop an
already-active capture. No GPU idle wait is added. An abrupt process kill or panic
without orderly render-world teardown cannot guarantee a finished document;
discard traces without a complete sidecar. A `complete` sidecar means capture
stopped successfully, **not** that Xcode has replayed the document.

This is a **render-update capture**. Other device/background work may appear;
commands created before capture start may be absent. It is not a promise of an
isolated GPU frame. Capture changes runtime behavior, so never use capture-enabled
runs as benchmark timings. Keep using the ordinary [Metal flights](README.md#gpu-flights-metal)
for performance measurements.

## Xcode inspection and compatibility

Open the resulting `.gputrace` in Xcode and inspect the submitted command buffers,
render encoders and presented color attachment. Record the Xcode and macOS
versions, visible pass/draw evidence, and a screenshot proving that the captured
scene and drawable match the ready screenshot. Bound any open/replay wait; a file
on disk alone is insufficient evidence of viewer compatibility. Do not claim
bring-up complete before a real fullscreen Maybraid document has been inspected.

Programmatic capture is proven both by a standalone Swift probe and by a real
foreground fullscreen Maybraid run on macOS 27.0.1 / Xcode 16.4. The game produced a
nonempty GPU trace document with requested/start/stop update2400, zero waiting or
failed pipelines, native1920×1080 screenshot/window metadata, successful fixtures
and clean exit0. The first AX-gated attempt never triggered and remains invalid.

Xcode inspection remains blocked: LaunchServices rejected `/Applications/Xcode.app`
before document load with error−10664 (`kLSIncompatibleApplicationVersionErr`, app
incompatible with the current OS). Therefore no command-buffer/draw/presented-color
inspection or replay acceptance is claimed. A compatible viewer is still needed.
The `gpucapture` CLI separately hung during discovery for both game and probe;
this programmatic route does not depend on it. Do not repeatedly retry that CLI or
bypass the Xcode compatibility rejection.

All output remains under ignored `profiling/captures/`. Raw captures are not
published by the harness; publication requires explicit authorization and follows
the repository's [Traces release](../CONTRIBUTING.md#traces-releases) conventions.

An armed request currently retains a non-Send run-state resource until exit. If
it is never triggered, Bevy can warn that main-thread teardown forgets that state;
no native capture has started, so this cannot stop or corrupt another capture.
Successful/failed triggered runs remove their state on the render thread. A later
cleanup change can separate Send run/status state from the native session; that
engine refactor is deferred while the first capture is validated.
