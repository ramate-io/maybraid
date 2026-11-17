# Available-first generation and place derivation: November 16 validation

Investigated world XZ `(1746.8, 1037.9)` on maybraid-gpu. Baseline main was
`068c222bf0a8da68742446085563550520726e0a`; the candidate contains the accompanying
generation/readout, place derivation and idle lid changes. The Bevy fork is unchanged.

## Method and results

Four diagnostic recordings ran in order A1 main, B1 candidate, B2 candidate,
A2 main, using immutable binaries within each arm. Each passed the foreground
1920×1080 fullscreen guard, supported initial position/view, 40-second static
settle and approximately 131-second movement/fire window (410 shots).
VSync off/Immediate was requested; uncapped physical display throughput is not
established. Trajectories vary slightly; the replays are not pixel-identical.

| Work, milliseconds | A1 | B1 | B2 | A2 |
| --- | ---: | ---: | ---: | ---: |
| Matched DevelopmentPlaces derivation | 4830.338 | 17.127 | 20.144 | 5063.311 |
| Startup journey selector maximum | 165.013 | 0.365 | 0.464 | 184.453 |
| Movement journey selector maximum | 4.342 | 0.126 | 0.125 | 52.872 |
| Movement main-work p95 | 18.907 | 19.156 | 19.025 | 19.307 |
| Startup pipeline-queue maximum | 259.298 | 3437.295 | 247.654 | 250.212 |
| Movement pipeline-queue maximum | 29.259 | 360.483 | 27.997 | 28.269 |

The exact DevelopmentPlaces scheme and development bounds match across runs;
the enclosing preload requests differ and are not compared as identical work.
Cached Built output and selected dependency coverage were audited. Neither
candidate contains generation nested under the journey selector. In A2, an
Urban tile lookup builds Built on that thread for 52.534 ms inside a 59.278 ms
main-work frame. The candidate consumes available publications instead.

B1's largest frame gaps occur in render-side pipeline processing. They do not
repeat in B2, supporting a first-run cache-warming hypothesis without identifying
the compiler/driver/cache cause. Steady p95 differences overlap and change
direction. These runs establish the targeted improvements, not a broad FPS or
GPU speedup or universal absence of regressions.

Publication reads now return at most 32 IDs. In the first pair's movement
window, presentation-read maximum falls from 0.811 to 0.048 ms while total
bookkeeping rises from 7.708 to 11.234 ms. Initial host applies total 6,668 in
both runs. This bounds copying, not every spatial-index query/reconciliation.

## Memory leads

A separate **plain main** travel/return/Home run completes all support gates.
Final 30-second median physical footprint is 6.225 GiB at initial P0, 5.041 GiB
at the second return, and 4.317 GiB at Home. Home is still falling, so that is
not a settled floor. RSS remains high despite allocation/count reductions;
RSS alone does not prove a leak.

A separate **candidate, census-only** run samples supported P0 and Home:

| Selected accounting | P0 | Home |
| --- | ---: | ---: |
| Live main-world entities (including resources) | 988,391 | 17,515 |
| Initialized component inline bytes | 249,941,674 | 5,210,318 |
| Mesh3d references / unique IDs | 208,780 / 5,968 | 769 / 99 |
| CPU mesh logical / capacity bytes | 422,252,088 / 481,742,536 | 422,331,036 / 481,821,484 |
| Terrain / water cache entries | 604 / 92 | 604 / 92 |

P0 has about 986,000 transform pairs and 770,000 Children components. Prioritize
attribution of reserved ECS columns, nested allocations and source-scene owners,
then identify owners of Home-retained mesh IDs. Meshes already share substantially
(about 35 references per unique ID). These counts do not justify a geometry
packing rewrite by themselves. Mesh payload remains about 403 MiB at Home;
cache payload overlaps it and is not additive. Nested heaps, full render-world
storage, GPU residency and allocator overhead are outside this census. Neither
run establishes a memory-saving A/B or smaller-device fit.

## Validation and evidence identity

Passed: HCSG 93 tests in each of ordinary/generation-tracing configurations
(8 ignored fixtures each), POI 47 (3 ignored), journey 12, world POI 8, and
focused census/cache/parser/lid/ordered-place-parity tests. The app check with
all diagnostic features and plain, Tracy/generation, and census-only release
builds passed. The isolated 449-pin place fixture preserves exact output and
improves 21.44 to 1.26 ms/call; this is distinct from the runtime measurement.

Diagnostic baseline binary SHA256:
`91ff01c22ddcf0e6bce0b8b15862f328f9a2f32500ee26bda7e75a81f43ec321`.
Candidate diagnostic binary SHA256:
`95e74d000a0981dc166895d1d887deb14df7eaecc346a3fc3f414db4d80093bd`.
Candidate source manifest SHA256:
`ab1326a0b06d521e88159f6305d610cc40d29b3d65761e5d5f95f9aa476bc987`.

Raw captures and detailed analysis are local under the ignored
`profiling/captures/area-1746-1037-20261116/` directory; they have **not** been
published with this PR. `implementation/RESULTS.md` indexes tests, source proof,
four-run analyses, pipeline attribution, and memory reviews. Accepted runs are
`main-natural-generation-capture/runs/natural-B-r5` and, under
`implementation/runtime-validation-r1/`, `candidate-natural-run-r2`,
`candidate-natural-repeat-r1`, and `main-natural-repeat-r1`. Failed attempts are
preserved and excluded. All owned runtime processes exited.
