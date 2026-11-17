# Pipeline identity diagnostics

Build the game with `--features pipeline-profile,bevy/trace_tracy` and enable
`maybraid.pipeline.profile=trace` in `RUST_LOG`. The default game does not compile
or install this diagnostic. Use it separately from clean timing recordings.

After render submission, the observer records each newly visible cache descriptor
and subsequent readiness transitions. Descriptor data includes pipeline labels,
shader handles and available paths, entry points, shader definitions and render
state. `(epoch, pipeline)` is a **run-local** identity, not a cross-run hash or a
serializable prewarming recipe. Shader hot reload can return an existing identity
to a waiting state. An error may be retried by Bevy; it is not reported as ready.

Correlate these events with the existing `process_pipeline_queue_system` Tracy
zone in the same render update. On macOS a pipeline may already be ready at its
first observation because creation is synchronous. Multiple variants can become
ready together: the queue duration cannot then be assigned to a particular one.
These events do not measure per-pipeline compilation time or GPU time. Newly
queued descriptors are private until Bevy drains them, so an empty waiting count
is **not** a global loading/readiness barrier.

Use the inventory to select actual first-use variants for a focused loading-time
prewarming experiment. This patch does not alter loading policy, queue extra
variants, change Metal compilation threading, or claim a hitch reduction.
