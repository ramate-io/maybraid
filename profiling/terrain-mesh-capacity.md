# CpuShot final mesh capacity

CpuShot now reserves the actual already-materialized cube vertex/index totals
when skirts are disabled, then publishes compact position/normal/UV/index Vecs.
Skirt meshes keep the original growth path before final compaction; reserving only
core geometry first caused avoidable growth followed by shrinking. Checked totals
and failed exact reservations fall back to original growth, never a missing mesh.
No geometry, mesh identity, asset usage, cache policy or wall traversal changes.

The earlier candidate census measured **54.59 MiB** capacity minus logical payload
across the terrain/water mailbox reports (53.67/0.93 MiB). That is an observed
spare-capacity ceiling for those assets, **not measured global memory savings**.
Newly generated CpuShot meshes should lose that reported spare element capacity;
warm cache loads may already be compact. Boxed-slice conversion can reallocate
and does not guarantee physical allocator release or a corresponding RSS change.

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
validation. No whole-game build/capture or OS memory saving was measured here.
