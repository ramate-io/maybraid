# Finalize Les Halles structure before fitting interiors

[Issue #1610](https://github.com/ramate-io/maybraid/issues/1610) tracks this change.
Ordinary Les Halles developments and RingFort curtains fitted their interiors,
then rebuilt and repainted the last storey to add a ceiling. Low window density
could cause every storey to be repainted. RingFort subsequently removed that
ceiling to leave the gallery open under its terrace deck.

The development now finalizes window density before shaft selection and supplies
its final top-slab policy to the initial monotower fit. Each storey's usage is
painted once. Ordinary developments retain a solid top gallery ceiling; curtains
are open for the caller's terrace deck. Standalone monotower fitting retains its
existing no-ceiling policy. This removes the second reconstruction of confines,
openings and usage, plus RingFort's add-then-remove ceiling work.

Legacy layouts are not preserved exactly. Finalizing density earlier may change
shaft choices or feasibility, and retaining the initial opening ownership avoids
a second floating-point reconstruction. Tests enforce the intended structural
properties rather than old furniture positions. Existing Commercial/Livable
usage nonrepeatability is tracked separately in
[issue #1611](https://github.com/ramate-io/maybraid/issues/1611).

## Why this target

Generation traces around player XZ (1746.8, 1037.9) showed long Built realization
operations. Two RingFort plans reproduced that cost headlessly: roughly 46–50 ms
and 76–77 ms. Stage diagnostics attributed about 21–22 ms and 35 ms respectively
to the repeated last-storey Livable paint. Rebuilding the floor plan itself cost
only about 54–66 µs. Required fitting is already worker work; this change reduces
its cost rather than moving it to another thread or changing readout semantics.

The plans below use seed 1337, fixed published terrace heights, and the exact XZ
bounds identified in the traces. A third traced cell produced an Empty plan in
the headless setup despite successful runtime Built publication; it is excluded.
The trace/stage evidence remains local under
`profiling/captures/area-1746-1037-20261116/` and the original diagnostic logs.

## Repeated complete-fit comparison

Measured 2026-11-17 UTC on maybraid-gpu, Apple M4 Pro, 48 GiB RAM, in the Nix
release toolchain. Source base: `2ee3646947ea179e36d5956f1a51bbfd31edaf19`.
The later parent commit adds only the memory report. Both arms run in the same
release test binary/process, with a scoped test-only legacy policy. That policy
restores the original initial fit, full density/last-storey repaint, and original
RingFort ceiling removal. The production build contains none of that policy or
its instrumentation.

Each repetition warms both arms, then alternates arm order across eight batches
of eight complete fits per arm: 64 samples per fixture per arm. Immutable Terrace
setup and output destruction are outside timing; fitting allocations are inside.
The p95 is nearest-rank over individual fits, not batch averages.

| Fixture | Repeat | Legacy median / p95 (ms) | Candidate median / p95 (ms) |
| --- | ---: | ---: | ---: |
| 0 | 1 | 47.548 / 49.256 | 24.014 / 25.153 |
| 0 | 2 | 46.016 / 48.451 | 23.271 / 23.502 |
| 1 | 1 | 76.006 / 79.605 | 38.437 / 40.905 |
| 1 | 2 | 78.354 / 79.331 | 39.558 / 40.622 |

- Fixture 0: XZ min (1998.8796, 1130.1075), max (2400, 1610.566), terrace height
  35.708984; both arms produce 3 ring storeys and 28 keep storeys.
- Fixture 1: XZ min (800, 1610.566), max (1104.924, 2086.4426), terrace height
  −193.13463; both arms produce 3 ring storeys and 30 keep storeys.

Median complete-fit cost falls approximately **49–50%** in these two cases.
A separate 24-seed corpus accepts 24/24 ordinary Les Halles and 24/24 RingForts
in each arm, with identical paired floor counts and no success/failure changes.
That bounded corpus does not prove unchanged feasibility for every seed, nor does
it prove complete furniture identity. No new runtime/GPU A/B, navigation or visual
acceptance pass was performed for this change; no FPS or arrival-latency claim
follows from these worker realization measurements.

## Validation and reproduction

- Full release `buildings` library: 435 tests passed.
- Full release `urbanization-developments` library: 76 passed; the benchmark was
  ignored. A second ignored workload census was subsequently added and passed.
- Focused checks cover one shell sample and usage paint per storey, top slab
  policy, explicit inbound shaft IDs and bounds, vertical shaft alignment through
  density repair, floor program types, stairs/deck holes, finite plans, and errors
  for tiny confines.
- Production `cargo check --release --offline -p richmond` passed.

From the repository Nix shell:

```sh
cargo test --release --offline -p buildings --lib
cargo test --release --offline -p urbanization-developments --lib
cargo test --release --offline -p urbanization-developments --lib single_paint_full_fit_benchmark -- --ignored --nocapture --test-threads=1
cargo test --release --offline -p urbanization-developments --lib single_paint_workload_census -- --ignored --nocapture --test-threads=1
cargo check --release --offline -p richmond
```

Repeat the benchmark command for an independent repetition. The test-only legacy
arm is a performance control, not a selectable production mode. Raw per-fit samples,
workload records, source patch and test logs remain local under
`profiling/captures/urbanization-single-paint-20261117/`; they are not uploaded.
