# Metal GPU measurement provenance

The earlier Metal GPU timings came from **Instruments Metal System Trace**, not
Tracy GPU zones. Keep these three capture types distinct:

| Capture | What it establishes |
| --- | --- |
| Tracy `.tracy` on our Metal builds | CPU zones and application frame intervals, including render preparation and CPU waits. A render or culling zone name does not make it GPU execution time. |
| Instruments `.trace` / `.trace.zip` | Game-PID GPU execution intervals, encoder labels and GPU performance states. This recorder adds overhead. |
| Xcode `.gputrace` | Inspection of a captured render update: draws, resources, pipelines and shaders. Capture-enabled runs are not clean benchmark timings. |

## Why Tracy has no Metal GPU zones

At our validated Bevy pin
[`dfbb7d0`](https://github.com/ramate-io/bevy/blob/dfbb7d045b15879232dde2efc9bc66f499413ef6/crates/bevy_render/src/diagnostic/tracy_gpu.rs),
`new_tracy_gpu_context` returns `None` for Metal before initializing timestamps.
This is an inherited
[upstream startup-freeze workaround](https://github.com/bevyengine/bevy/commit/08eaaec0110dcc9441786c9c1bddd1b735f430a6),
also present in our earlier dirty-upload fork revision. It is not a new removal
of a GPU measurement facility we previously used. `trace_tracy` still records CPU
spans; Apple's external Metal tracing works independently of this Bevy gate.

## Where the earlier numbers came from

The November 9 area-976 POI experiment attached `xctrace` to the live game for
25 seconds, with a Tracy collector also connected. The reported opaque bucket
means **9.746 ms (A) and 10.301 ms (B)** came from Instruments GPU intervals in
opaque-present groups. They were not CPU opaque-pass durations or Tracy GPU
timestamps. See the published [A companion](https://github.com/ramate-io/maybraid/releases/download/v0.1.5-traces/24a143b_20261109_m4pro-macmini_area-976-poi-a-gpu-metal-r1.md)
and [B companion](https://github.com/ramate-io/maybraid/releases/download/v0.1.5-traces/e83ac90_20261109_m4pro-macmini_area-976-poi-b-gpu-metal-r1.md).

The corrected analysis aligned UTC scenario Start/End log lines with each
Instruments TOC `start-date`, then compared a common **23.5113-second** window.
Game-start plus simulation time was rejected because Bevy clamps large deltas.
GPU intervals overlap: use their union for busy time rather than adding pass
durations. Instruments group counts are not presented FPS. Partial clock-state
coverage, attachment artifacts and different natural populations prevent a
causal GPU performance claim from this single pair.

## How to measure now

Use the [Metal flight harness](../profiling/metal-flight.sh) and
[table parser](../profiling/metal-passes.py) as the starting point. The essential
attach operation, after the scenario starts, is:

```sh
/usr/bin/xctrace record --template 'Metal System Trace' \
  --time-limit 25s --output "$trace" --no-prompt --attach "$game_pid"
/usr/bin/xctrace export --input "$trace" --toc
```

Export `metal-application-encoders-list`, `metal-gpu-intervals`, and
`gpu-performance-state-intervals` from the trace. Retain the game PID, TOC, UTC
scenario boundaries, source/binary identity and recorder logs. Check the parser's
window selection: its legacy simulation-duration/first-GPU-event fallback does
**not** implement the corrected UTC alignment above. Verify or correct that
alignment before treating its summary as a matched A/B result.

Run matched fullscreen, VSync-off Tracy-only walks for CPU and pacing comparisons,
then separate shorter Instruments A/B captures for GPU attribution. The earlier
joint recording remains instrumented evidence, not a clean Tracy-only baseline.
Bound recording **and finalization/export** time; a recording that never finalizes
is not a valid GPU comparison. Retain failures and do not substitute CPU zones for
missing GPU intervals. Check encoder-label and performance-state coverage.

For draw/shader inspection use the separate
[GPU frame capture harness](../profiling/gpu-frame-capture.md), which builds with
`gpu-frame-capture` and launches with `MTL_CAPTURE_ENABLED=1`. That switch enables
Xcode capture; it does not enable Tracy Metal timestamps.
