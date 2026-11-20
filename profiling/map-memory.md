# Offline map-run process memory

`summarize-map-memory.py` reads a completed, independently admitted map capture
and writes `summary.json` and `TABLE.md`. It uses only Python's standard library
(Python 3.9 or newer); it launches no application, queries no OS memory API and
requires no flight harness or trace exporter.

```sh
python3 profiling/summarize-map-memory.py ./accepted-run ./new-memory-summary
python3 profiling/test-summarize-map-memory.py -v
```

The output directory must be new. Invalid core input produces `rejected.json`
and a nonzero exit; the input records are never modified. Keep actual captures,
logs, metadata and generated summaries outside Git.

## Input contract

The run directory contains:

| File | Required data |
|---|---|
| `completed.json` | `normal_exit=true`, `distance_goal_reached=true`, and the SHA-256 of `binding.json` in `binding_sha256` |
| `binding.json` | The capture's immutable provenance record |
| `collector-game-pid.json` | Game `pid` agreeing with the launch record |
| `memory.csv` | `utc`, `elapsed_s`, `pid`, `process_start_abstime`, `rss_bytes`, `phys_footprint_bytes` |
| `lifecycle.jsonl` | One UTC `game_launched` with PID, one successful UTC `game_exited`, and `pilot_checkpoint` observations 52 and 104 with UTC and player pose/vitals |
| `game.log` | UTC `agent.scenario NAME Start/End` lines for `settled`, `hitch-walk`, `living-map` and `respawn` |
| `events.jsonl` | Matching unique scenario start/end events |

The sampler format is the macOS process-ledger format; this offline reader runs
on any supported Python host. It requires one positive process-start identity,
matching PID receipts, monotonic timestamps and an observed median cadence of
4.5–5.5 seconds. Gaps over 7.5 seconds and insufficient lifetime/settled/walk
coverage reject the summary. Phase alignment uses actual UTC log markers,
not simulation time interpreted as wall time.

## Output and limits

The table separates initial settled state, walking, moving windows within
10 seconds of revisits 52/104, living-map, respawn and the whole process.
It reports sample counts and footprint/RSS medians and sampled maxima; raw byte
values, peak UTC, identity and input hashes remain in JSON. Short observer
phases with no adequate samples have explicit unavailable/null metrics rather
than invented zeroes. Checkpoint support is an observation, and actual camera
pose is not inferred there.

Five-second sampling can miss shorter peaks. Whole-process peaks include
startup, screenshots and map/death activity; use the walking row for ordinary
travel. Revisit windows are moving observations, not settled returns or proof
of matching rendered content. These are process counters, not an ownership,
leak, GPU-residency or optimization-savings measurement. Admission of geometry,
foreground, health and the capture itself remains the caller's responsibility.

## Bounded gameplay ownership follow-up

Keep the existing capture protocol unchanged. In a separately reviewed next
measurement, reuse a proven outbound corridor, retrace it fully to the actual
recorded initial supported player position, wait 40 seconds there, then revisit
the same far checkpoint. Use ordinary movement and the original actor, health,
gravity and support/readback gates; stop on obstruction or a finite deadline.
Do not substitute Home teardown, map/death, teleport or pose overrides for a
return during active gameplay.

Take explicit read-only ownership snapshots at initial settled origin, first
far arrival, settled far, quiet origin return and settled far revisit. Compare
ECS live rows versus empty/partial/full reserved column capacity, Mesh/Image CPU
payload and live references, plus retired/pending/cull/visibility work counts.
Record content/camera and backlog so quiet time is not mistaken for quiescence.
Keep each census outside timing observations and tag its duration; diagnostic
features and the census itself can affect process memory.

For an approximately one-kilometre centerline corridor, three passes imply
roughly three kilometres plus steering overhead. Prespecify enough time for
ordinary movement, the quiet epochs and cleanup; do not widen the run after a
failed gate. Stable live content with growing reserved capacity supports a
bounded capacity investigation. Persistent work debt or growing live assets
points elsewhere. If selected scopes stay flat while process memory rises,
leave the residual unattributed. No periodic pruning, storage conversion,
retirement policy or memory mutation is implemented by these tools.
