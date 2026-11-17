# CpuShot final mesh capacity

CpuShot now reserves the actual already-materialized cube vertex/index totals
when skirts are disabled, then publishes compact position/normal/UV/index Vecs.
Skirt meshes keep the original growth path before final compaction; reserving only
core geometry first caused avoidable growth followed by shrinking. Checked totals
and failed exact reservations fall back to original growth, never a missing mesh.
No geometry, mesh identity, asset usage, cache policy or wall traversal changes.

A fullscreen cold application-cache ABBA now measures **57,244,768 B (54.5929 MiB)
less live main-world CPU mesh Vec element capacity**, repeated at P0 and Home.
Logical payload and mesh counts match across both arms. This is an application
buffer-capacity result, **not an established RSS or physical-footprint saving**.
Boxed-slice conversion can reallocate; allocator-resident bytes need not follow
Vec capacity, and warm decoded payloads may already be compact.

## Focused verification

Two tests passed: real plane and stepped SDFs on an offset, non-cubic chunk with
no/selective/all skirts compare legacy versus candidate serialized attributes and
indices bit for bit, plus topology, asset usage and geometric validity. They also
verify final element capacity equals initialized payload. The test-only legacy
allocation branch uses the same geometry pipeline with the old growth policy.
A bincode `SerializedMesh` payload roundtrip preserves geometry/usage and decodes
compact Vecs for the fixture, including a legacy-built mesh. This exercises the
disk cache's payload representation, not filesystem eviction, compression I/O or
every possible decoded mesh. No disk key/version change is needed.

The production configuration also passes `cargo check --release --offline -p render-item`.

## Allocation and time comparison

Release test binary; fixed `RAYON_NUM_THREADS=1`; stepped SDF; resolutions 16/32
cubes per axis. Each case warms both modes, then alternates order for 30 paired
timing iterations with allocation counting disabled. A separate three-pair pass
counts successful System allocator requests, including Rayon work. These counts
are allocation traffic, not net/peak heap or bytes copied. The disabled counting
allocator still has its common flag check in both timed modes.

| Resolution /skirts | Logical B | Capacity B old→new | Median µs old→new | p95 µs old→new |
| --- | ---: | ---: | ---: | ---: |
|16 /none|49,856|63,104→49,856|114.458→114.646|132.208→137.666|
|16 /all|59,584|89,344→59,584|116.813→116.521|121.209→141.416|
|32 /none|193,648|249,376→193,648|516.292→514.938|525.000→525.417|
|32 /all|213,104|351,296→213,104|498.959→499.625|537.875→543.375|

p95 uses nearest rank. Medians are essentially unchanged in this fixture; the
small skirt case has a higher tail. This does **not** prove no generation/arrival
regression, parallel production behavior, or any whole-game frame improvement.

| Resolution /skirts | Reallocations old→new | Requested B old→new |
| --- | ---: | ---: |
|16 /none|385→367|428,632→378,592|
|16 /all|387→391|481,112→540,696|
|32 /none|1,379→1,357|2,619,928→2,416,816|
|32 /all|1,381→1,385|2,823,768→3,036,872|

Allocation calls stay 689/2,597 for resolutions 16/32. No-skirt reservations avoid
18/22 growth reallocations; skirt finalization adds four shrinking reallocations
and additional traffic equal to logical mesh bytes. Compaction reduces retained
reported capacity by ~21–39% in these fixtures, with that explicit work tradeoff.
Transient grid, per-cube Vecs and coordinate lists remain unchanged.

Reproduce inside the repository's correct Nix shell/native clang environment:

```shell
RAYON_NUM_THREADS=1 cargo test --release --offline -p render-item sdf::cpu_shot::tests -- --test-threads=1
RAYON_NUM_THREADS=1 cargo test --release --offline -p render-item sdf::cpu_shot::tests::capacity_cost_comparison -- --ignored --nocapture --test-threads=1
```

Raw output remains local, not uploaded: `profiling/captures/terrain-mesh-capacity-20261117/r3-tests.log`
and `r3-benchmark.log` in the same directory. Earlier r1 counted allocations inside its timed region and varied
Rayon scheduling; r2 retained that old benchmark after a preparation-path error.
Both remain in `/private/tmp/maybraid-capacity-check-r{1,2}` and are not timing
validation. Fullscreen memory results follow; the focused test supplies no whole-game timing claim.

## Fullscreen memory ABBA

Measured on 2026-11-17 UTC: Apple M4 Pro (12 CPU cores), 48 GiB RAM,
macOS 27.0.1, integrated Apple M4 Pro GPU using Metal (recorded startup system
and adapter information). Fixed P0 XZ **1746.8, 1037.9**, camera yaw/pitch
−2.6/−0.15; supported arrival, no travel/fire, 90 s P0 settle, then Leave to Home
and 120 s settle. The predefined extension adds 60 s when needed (Home ≤180 s).

Two runs per arm, in A/B/B/A order, used clean parent `e91ab541c2fb440802e6d0948d9209ad03e5a6fb`
and candidate `2ee3646947ea179e36d5956f1a51bbfd31edaf19`, both with only `memory-census`.
Binary SHA256 A: `5875cb69036e30302d413ff1a754f5ecfbf28af1a0e9403eb3ebb12753aaa223`;
B: `23d1d6c0c25a71746156d2c082ce0888a1291700eab0d2f52c8c266ad1ab96e5`.

All four completed with clean owned-process cleanup, supported arrival, fullscreen
1920×1080 and VSync requested off/Immediate. Assets were shared/absolute; saves,
logs and initially empty application mesh-cache CWDs were isolated per run. OS and
driver caches were not reset. This is a relative instrumented memory comparison,
not an ordinary-build absolute budget, GPU-residency census or CPU benchmark.

Each selected phase has three matching snapshots five seconds apart, after its OS
window. All four runs match 604/92 terrain/water entries, unique-ID and CPU-asset
counts, plus logical payloads. Global CPU Mesh count/logical/capacity triples are
stable independently, not inferred by summing cache reports:

| Phase | Assets /logical B, all runs | Capacity B A1/A2 | Capacity B B1/B2 | Reduction |
| --- | ---: | ---: | ---: | ---: |
| P0 |6,001 /422,252,088|481,742,536|424,497,768|54.5929 MiB|
| Home |6,085 /422,331,036|481,821,484|424,576,716|54.5929 MiB|

P0 mesh-buffer capacity is 11.88% lower. **OS memory savings are indeterminate**:
B1 P0 still drifts beyond the predeclared 10 MiB gate after its bounded extension;
its extra scan/elapsed history also precedes Home. RSS did not fall in the paired
observations. Home footprint was lower in B, but repetition varied by 332.52 MiB
within A and 224.09 MiB within B, exceeding the capacity signal. These distinct
ledgers cannot be added to capacity bytes or treated as a guaranteed saving.

Full local evidence, not uploaded, is under
`profiling/captures/terrain-mesh-capacity-20261117/memory-abba/`: `RESULTS-R2.md`,
`results-r2.json`, `analysis-final-r2.json` and `runs-r2/{A1,B1,B2,A2}/`.
The stale shared-target `build-candidate/` copy and failed pre-P0 `runs/A1` pilot
were excluded; admitted B uses the independently rebuilt `build-candidate-r2/`.

The local frozen harness is `memory-abba/prepared-r2/{A1,B1,B2,A2}/abba-flight.py`
under that evidence directory. To reproduce one baseline arm with fresh paths
(and repeat B/B/A with `build-candidate-r2`/`build-base` manifests and distinct labels):

```shell
P=/Users/l-monninger/.codex/worktrees/performance-main/maybraid-gpu/profiling/captures/terrain-mesh-capacity-20261117/memory-abba
python3 "$P/abba-flight.py" prepare "$P/build-base/maybraid.manifest.json" --output "$P/repro-prepared/A1" --run-output "$P/repro-runs/A1"
cd /Users/l-monninger/dev/maybraid-gpu
/usr/bin/caffeinate -dimsu nix develop /private/tmp/maybraid-area-1746 --command python3 "$P/repro-prepared/A1/abba-flight.py" run "$P/repro-prepared/A1/config.json" --output "$P/repro-runs/A1" --lease FRESH_SERIAL_ROOT_LEASE
```

This requires the pinned local source worktrees, immutable builds and tools in
the manifests, an unlocked display and an exclusive runtime lease. Existing
output/config paths are rejected, never overwritten; no user cache is deleted.
