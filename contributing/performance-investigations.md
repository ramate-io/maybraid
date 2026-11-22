# Performance investigations

Start from a measured workload and change one thing at a time. Use the existing
[profiling tools](../profiling/README.md) and capture scripts. Keep a small script,
read-only probe, or analysis change scoped to what the existing tools cannot see.

## Run comparable native captures

- Use the same executable with a runtime toggle where possible. Record the tested
  source revision, dependency lock, effective flags, binary hash, fixture, route,
  and observer settings. Identify each binary explicitly if a rebuild is needed.
- For uncapped native Metal measurements, match fullscreen resolution and render
  scale and request VSync off. Verify actual Immediate mode and that the game
  stays foreground at the intended display coverage. Record fallback or focus
  loss and qualify the affected capture.
- Keep the game playable. Use normal spawn, terrain support, gravity, collisions,
  health, and camera. Check the supported player state and realized content.
  Match routes, views, inputs, and sample windows. Checkpoint proximity alone
  does not establish equal actor, LOD, or draw workloads.
- On a shared benchmark machine, use one operator to schedule and hand off
  builds, native captures, and large exports. Verify the previous job exited
  before the next one starts. Put a deadline on each run and keep the display
  awake for the authorized window.

## Preserve what the capture proves

- Start with application frame intervals, tails, and relevant CPU scopes.
  Report the time window, gap counts, and what was included. Show startup,
  compilation, and observer-heavy phases separately. Apply equivalent masks
  across arms when reporting a filtered comparison.
- CPU render scopes measure CPU wall time, including any waits. Measure GPU
  execution separately before claiming GPU cost or headroom. For process memory,
  label the counter, sample interval, process identity, and phase. A sampled
  footprint maximum is distinct from an instantaneous peak or GPU residency.
- Preserve deaths, obstructions, and incomplete lifecycles in the run record.
  A verified common prefix can answer a narrower question if its cutoff is
  fixed before the paired run. Label that scope and keep the original failure.
- Publish a short comparison with the exact tested source, admissions, observed
  differences, and limitations. Keep raw captures and detailed receipts
  available for audit. Follow the [trace release guide](../CONTRIBUTING.md#traces-releases)
  when publishing a trace.
