# Grove-owned visual investigation

This continues [#1647](https://github.com/ramate-io/maybraid/pull/1647) and the partial replacement in [#1650](https://github.com/ramate-io/maybraid/pull/1650), following [#1651's investigation guides](https://github.com/ramate-io/maybraid/pull/1651). Implementation and native validation are in progress; no performance improvement is established by this document.

See the [later whole-grove native benchmark at f192490](grove-f192-native-benchmark.md) for the matched route, standing Metal and checkpoint census results.

## Catalog GPU encode (this continuation)

Per-grove `OwnerGpuState` is gone. Each vegetation kind keeps one catalog of grove records plus concatenated plant/branch/leaf (or blade) arenas. Extract drains `CatalogOp::{Admit,Retire}` with `mem::take` and never clones the field. Prepare grows on admission (phase A CPU growth of that grove only) and `write_buffer`s those ranges. Retire clears `FLAG_ACTIVE` without rebuilding.

Per view, per kind, encode is independent of grove count:

| Kind | passes | dispatches | draws | uniform writes |
|---|---:|---:|---:|---:|
| tree | 1 | 3 | 6 | 1 |
| bush | 1 | 3 | 6 | 1 |
| grass | 1 | 2 | 3 | 1 |

Five presenters (tree wood, tree foliage, bush wood, bush foliage, grass) each own a bin and carry `NoFrustumCulling` so CPU AABB does not hide GPU instances. GPU `ChicoGroveHost`s emit no `lod_host_scene_pending` and are not LodScene-indexed; the 1 km present ring only admits and retires. Bands still come from `lod.wesl` grove thresholds (High ~400 m, Medium ~520 m, Low 1 km). Compact lists grow to the admitted branch/leaf counts so one High band cannot overflow 8192 slots and drop foliage. `MAYBRAID_EXPERIMENT_TREE_FIELD=cpu` still uses CPU select. Bush/grass global caps are 262144 plants and 1048576 blades so admitted content is not silently truncated.

`--release` tests: catalog idle/admit/retire, encode counts identical for 10 and 1000 groves, `grove_gpu_far_bands_match_legacy_instance_counts`, `gpu_host_is_not_lod_scene_indexed`, `cpu_experiment_keeps_cpu_select_when_compute_is_available`.

### Native gates for the checker

On the Area 1746 walking windows, compared to the f192490 per-grove encode (N = 295–548 tree groves → N compute passes):

- Grove compute command-buffer finish must stay **under 0.3 ms** (was ~13.5 µs × N ≈ 2.3 ms walking / 6 ms standing).
- `queue_submit` count must be **at most legacy + 3** (one extra submit family per vegetation kind, not per grove).

Do not admit a GPU win if those gates fail, even if application interval looks better. Re-run the [native rerun spec](#native-rerun-spec-grove-owned-gpu-vs-legacy) on this catalog head.

## Frozen comparison

Latest main was fetched and audited at `05885cd3056c8e4724d2bbd6265ac41255c1a962`. The experiment head `3c8e6fec904365557e9a034cd870f35111627e09` already includes that main revision. Its only difference from measured executable source `aacc5c07f2f26286456067e383a5384a1e26d17a` is the benchmark report. The frozen baseline executable SHA-256 is `1bdb82448dad1633dbc33e68c1c5131e54926f5afd8276a575feaaf6e0299908`; Cargo.lock SHA-256 is `ca4dc755ca2bd3ca053345a0cb3b5cfc8864b22619420296103194b02e49cf74`. The published Bevy fork stays pinned to `dfbb7d045b15879232dde2efc9bc66f499413ef6`.

The candidate compares CPU and GPU selection of identical authored descriptors. Legacy kits and the earlier synthetic field are separate geometry comparisons: the new adapter consumes the original Storybook chain and canopy sites, while #1650 generated another skeleton. A difference against that older field cannot isolate descriptor ownership cost.

## Intended ownership and content contract

A persistent `ChicoGroveHost` owns one immutable visual snapshot, separate wood/foliage presenters, and independent upload buffers. CPU residency and coarse visibility control the owner; internal visual bands do not publish per-plant patches or rebuild unrelated groves. Authored `Arc<StorybookTree>` data is shared with collision ownership. Collision-only metadata stays under the original High/Medium roots to preserve collider activation and retirement; mixed tiles also retain those roots for unsupported species. The change removes supported visual variants, not the physics residency mechanism. Admission must succeed completely before suppressing legacy Storybook visual variants and canopy proxies. Capacity failures retain the legacy representation.

The opt-in natural area is still the selected nine 100 m tiles around `(-4950, 3150)`. Storybooks are the supported species. Braid Oaks, Rory forms, other grove types, and plants outside the selection retain legacy rendering. Their cost remains in all arms. Species, positions, counts, source identities, and collisions need validation throughout the route; fewer rendered trees is not an optimization result. The former 220 m field cutoff is insufficient when distant CPU canopy proxies are removed: the far band must cover complete grove residency.

All benchmark arms disable shadows because the field lacks equivalent casting and receiving. Results do not establish performance or appearance with ordinary shadows enabled.

## Native capture plan

Use Nix, native Metal, fullscreen 1920×1080, render scale 1, VSync off, and verify Immediate mode without fallback plus foreground display coverage. One operator serializes builds, games, captures, and large exports. Binaries, source/lock hashes, settings, fixture, route and tool versions are recorded per run. Tracy must match the pinned client's protocol; this baseline uses the locally verified Tracy 0.14.1 tools.

Immediate verification is bound to the [published fork's surface configuration](https://github.com/ramate-io/bevy/blob/dfbb7d045b15879232dde2efc9bc66f499413ef6/crates/bevy_render/src/view/window/mod.rs#L489-L529): an explicit Immediate request selects Immediate when available, otherwise Fifo and an INFO message `PresentMode Immediate requested but not available. Falling back to Fifo`. The chosen value is passed into surface configuration. Require the game's `MAYBRAID_VSYNC=off (Immediate)` receipt, active native rendering, and absence of that fallback prefix throughout the complete log. `RUST_LOG=info,wgpu=error` preserves these `bevy_render` INFO messages. Foreground and fullscreen coverage remain separate checks.

`profiling/grove-lifecycle.py prepare PATH` emits a proposed 3 km ordinary walking route starting at Area 1746 `(1746.8, 1037.9)` (the published longwalk-checkpoints-r2 origin; see [area-1746-performance.md](area-1746-performance.md)). Loading is at that xz, then a 600 m +Z approach (waypoints at 50 / 200 / 450 / 600 m), standing, 1.2 km departure through 900 m, then 1.2 km return and standing. The previous native route held `x=-4950`, about 7 km west of this neighborhood, and is no longer the default. Overlay the published longwalk-checkpoints-r2 script if the native checker has that asset. Scout the route before admitting paired results. Retain failures, deaths, unsupported motion and incomplete phase records. Report launch-to-supported readiness separately from script windows.

## Native rerun spec (grove-owned GPU vs legacy)

Run both arms on the same machine, same binary SHA, shadows off, 1920×1080 Metal Immediate, VSync off. Do not admit a pair if Immediate falls back to Fifo. Hold NPC activity fixed or mask it in the receipts (the first native pair on `f192490` had different NPC counts along the same path).

1. **Checkpoint census before timing.** `profiling/grove-lifecycle.py prepare --census PATH` stops at ~50 / 200 / 450 / 900 m. Record per-band instance counts (trees / bushes / grass, layers, sizes) versus legacy. GPU far bands must stay within about 20% of legacy High / Medium / Low; understory and grass must still be present at 200 m+ (not the old 90–110 m shader cutoff).
2. **Order pair.** One pass legacy then GPU, then a reversed pass GPU then legacy. Same shadows (`MAYBRAID_SHADOWS=off`), same window, same route script.
3. **Tracy.** Capture main-app, render-app, extract and `prepare_windows` p95s next to the application interval. Upload `.tracy` files and summaries to the Traces release and link them on the PR.
4. **Metal GPU-busy.** One `profiling/metal-flight.sh` run per arm; do not mix observer-heavy GPU traces into the CPU interval comparison.
5. **Env.** Legacy omits `MAYBRAID_EXPERIMENT_TREE_FIELD`. GPU uses `MAYBRAID_EXPERIMENT_TREE_FIELD=gpu`. A `=cpu` arm must log and encode `CpuSelect` (not `GpuDriven`).

CPU captures report application interval tails, visibility/LOD and command-application scopes, changed owner publications and upload bytes. Exact-process memory is sampled at one-second intervals from launch through exit; its maximum is a sampled process footprint, not instantaneous peak or GPU residency. Separate feature-gated census runs report component membership churn and retained archetypes. GPU execution is measured in separate Metal System Trace runs, with observer-heavy timings excluded from CPU comparisons.

## Baseline route validation

The frozen legacy arm completed all 15 walking waypoints and the full lifecycle on 2026-11-22 UTC. Sampled player positions cover 3.013 km. Approach, departure, and return lasted 108.395 / 220.878 / 217.048 seconds; both standing windows lasted 15.002 seconds. Supported readiness was observed 12.871 seconds after the operator's launch timestamp. All 1,190 player samples retained the same identity and 100 HP; six airborne samples occurred during ordinary walking, and no route-phase samples had pending support. The initial loading phase included one pending-support sample before the readiness gate.

The nine selected tiles contained 32 Storybooks among 62 woody plants. All repeated tile inventories retained identical counts and placement digests. Foreground/fullscreen verification admitted 287 samples with no loss after the first valid sample; startup had ten pre-window retries. The published-fork Immediate check passed. The game and collector both exited successfully, retaining a 1,838,783,595-byte Tracy capture.

At one-second cadence, 597 gameplay process samples reached a maximum physical footprint of 5,509.94 MiB and RSS of 4,649.70 MiB. These are sampled peaks in an instrumented run, not instantaneous or GPU-memory peaks. CPU frame statistics and candidate comparisons remain pending; this validation establishes the baseline workload only.

## Simple TreePatch swap (this continuation)

Admission no longer imports authored wood/canopy topology. Each eligible grove becomes one `TreePatch` with one seed, the storybook recipe, and samples at the existing Storybook roots. Oak and Rory kits stay on the legacy path. Physics metadata keeps an identity `Transform` so collider children inherit `GlobalTransform` (Bevy B0004). Toggle remains `MAYBRAID_EXPERIMENT_TREE_FIELD=gpu|cpu` versus omitted for kits. Both GPU presenters stay `NotShadowCaster`; the world trial still requires `MAYBRAID_SHADOWS=off`.

A release `--locked` nine-tile CPU path on this Linux VM (`TemperateLowerMassivesParams` 3×3, not the native Metal standing scene) reported:

| Arm | Plants | Kit High nodes (stick/foliage) | GPU branches | GPU leaves | Upload | CPU select |
|---|---:|---:|---:|---:|---:|---:|
| Legacy kits | 35 Storybooks among 73 woody | 35 / 35 | — | — | per-kit meshes | not timed here |
| GPU TreePatch | 35 at the same roots | — | 804 | 17,146 | 1,165,024 B | 0.336 ms one-shot compact |

Kit node counts are High `VegetationComponents` lists (one collection per tree), not expanded GLB entities. CPU select is a single `TreeSelector::compact` over the nine generated fields, not an application-interval tail. GPU execution, upload counts during a live world session, entity/archetype churn, peak gameplay memory, and Metal frame times were not measured on this machine. Shadows were identical (`off` / `NotShadowCaster`) in both arms of this check. Bush world groves are not swapped; `BushPatch::from_roots` is available for the same placement pattern.

## Matched playground standing scene

The tree-field playground captured kit and GPU groves from the same cameras with identical shadow settings: `CanopyShadowMode::Off` and the key sun's shadow maps disabled. This is a synthetic `TreePatch::grove` (9 plants, 216 branches, 4,487 leaves, 305,312 B one-shot upload), not the world nine-tile neighborhood. The adapter was llvmpipe (Mesa 25.2.8, Vulkan). Window 960×540, `PresentMode::AutoNoVsync`. Frame times are Bevy `Time::delta` over 36 settle frames after skipping 12 post-swap frames. They are application intervals on a software rasterizer, not GPU execution and not a native Metal world route.

| Shot | Camera | mean ms | p95 ms | max ms | entities | kit meshes | Mesh3d |
|---|---|---:|---:|---:|---:|---:|---:|
| grove-edge-legacy | GROVE | 249.059 | 250.000 | 250.000 | 5,679 | 5,186 | 5,188 |
| grove-edge-gpu | GROVE | 51.848 | 54.349 | 56.204 | 494 | 0 | 2 |
| grove-legacy | GROVE_FULL | 210.140 | 220.699 | 221.262 | 5,679 | 5,186 | 5,188 |
| grove-gpu | GROVE_FULL | 61.086 | 63.910 | 66.169 | 494 | 0 | 2 |

Kit `grove-edge` p95/max of 250.000 ms is Bevy's default `Time` max-delta clamp, so those kit intervals are a lower bound. GPU samples stayed under the clamp. Kit GROVE_FULL sits inside cheap-ball canopy; GROVE (edge) is the readable pair.

Screenshots and per-shot receipts:

- [`maybraid/world/layers/vegetation/tree-field-playground/captures/grove-edge-legacy.png`](maybraid/world/layers/vegetation/tree-field-playground/captures/grove-edge-legacy.png)
- [`maybraid/world/layers/vegetation/tree-field-playground/captures/grove-edge-gpu.png`](maybraid/world/layers/vegetation/tree-field-playground/captures/grove-edge-gpu.png)
- [`maybraid/world/layers/vegetation/tree-field-playground/captures/grove-legacy.png`](maybraid/world/layers/vegetation/tree-field-playground/captures/grove-legacy.png)
- [`maybraid/world/layers/vegetation/tree-field-playground/captures/grove-gpu.png`](maybraid/world/layers/vegetation/tree-field-playground/captures/grove-gpu.png)

Not measured here: native Metal/world frame times, Tracy, GPU pass duration, live descriptor re-upload counts, entity/archetype churn, peak gameplay memory, bush-field world swap, Oak/Rory GPU admission, or appearance/shadow parity with ordinary shadows enabled. Nix was not available on this Ubuntu VM; tests and the playground used `cargo --locked --release`.

## Whole-forest GPU recomposition (owner correction)

The Storybook-only swap above is not the requested experiment. Every grove kind now claims a GPU field from existing plant roots before any benchmark: canopy tiles become storybook `TreePatch` samples at authored heights, understory/small woody become `BushPatch`, tufts become `GrassPatch`. `ChicoGroveHost` emits empty kit chunks when those visuals are admitted, so Oak, Rory, and the rest do not keep legacy meshes. Omitting `MAYBRAID_EXPERIMENT_TREE_FIELD` still presents the original kits.

A release `--locked` census of the world-route 3×3 around `(-4950, 3150)` (nine tiles × TallGrass + LowBush + Temperate Lower Massives) admitted 27 hosts, 78 storybook trees, 1,524 bushes, and 20,120 grass samples. High kit chunk weight was 27 (one empty primitive per host). `gpu_claim_swaps_every_grove_kind_and_drops_kit_chunks` grows all 41 `ForestGroveKind` values through the same host path.

### Matched stacked-forest standing scene

The playground grows the same stacked kinds, translated to the origin so cameras can stand outside the canopies: 2×2 Temperate Lower Massives, one LowBush tile, one TallGrass tile. Shadows stay matched off (`CanopyShadowMode::Off`, key sun shadow maps disabled). The adapter is still llvmpipe (Mesa 25.2.8, Vulkan) on this Ubuntu VM — a software rasterizer, not a discrete GPU and not native Metal. Window 960×540. Frame times are Bevy `Time::delta` over 36 settle frames; kit p95/max of 250.000 ms is Bevy's max-delta clamp (a lower bound).

GPU tree descriptors for this scene: 40 plants / 952 branches / 19,324 leaves, 1,316,736 B upload, plus 149 bushes and 2,220 grove grass samples. The grass catalog also still holds 24,336 blades from the playground turf cover (`gpu-grass-cover`), so the capture receipt's `grass=26556` is cover + grove tufts, not grove-only.

Refreshed after catalog encode (one pass per kind, grove_tree bands, lists sized to admitted leaves). Far GPU keeps the ridge instead of culling at the playground 220 m card. Far p95 rose on llvmpipe because High-band foliage now submits. Kit p95/max of 250.000 ms is still Bevy's max-delta clamp (a lower bound). Floor shots were not retaken.

| Shot | Camera | mean ms | p95 ms | max ms | entities | kit meshes | Mesh3d |
|---|---|---:|---:|---:|---:|---:|---:|
| forest-vista-legacy | VISTA | 250.000 | 250.000 | 250.000 | 22,039 | 21,533 | 21,536 |
| forest-vista-gpu | VISTA | 94.623 | 95.523 | 95.839 | 512 | 0 | 3 |
| forest-edge-legacy | EDGE | 250.000 | 250.000 | 250.000 | 22,039 | 21,533 | 21,536 |
| forest-edge-gpu | EDGE | 80.077 | 80.900 | 81.906 | 512 | 0 | 3 |
| forest-floor-legacy | FLOOR | 250.000 | 250.000 | 250.000 | 22,034 | 21,533 | 21,536 |
| forest-floor-gpu | FLOOR | 24.643 | 27.328 | 28.550 | 515 | 0 | 3 |
| forest-far-legacy | FAR | 250.000 | 250.000 | 250.000 | 32,541 | 32,035 | 32,038 |
| forest-far-gpu | FAR | 140.984 | 153.692 | 155.267 | 518 | 0 | 3 |

GPU `kit_meshes=0` is `KitPreview` count. `Mesh3d=3` is the forest ground plane plus two hidden turf stand-ins, not vegetation kits.

Screenshots (960×540) and receipts, committed with this revision:

- [`maybraid/world/layers/vegetation/tree-field-playground/captures/forest-vista-legacy.png`](maybraid/world/layers/vegetation/tree-field-playground/captures/forest-vista-legacy.png)
- [`maybraid/world/layers/vegetation/tree-field-playground/captures/forest-vista-gpu.png`](maybraid/world/layers/vegetation/tree-field-playground/captures/forest-vista-gpu.png)
- [`maybraid/world/layers/vegetation/tree-field-playground/captures/forest-edge-legacy.png`](maybraid/world/layers/vegetation/tree-field-playground/captures/forest-edge-legacy.png)
- [`maybraid/world/layers/vegetation/tree-field-playground/captures/forest-edge-gpu.png`](maybraid/world/layers/vegetation/tree-field-playground/captures/forest-edge-gpu.png)
- [`maybraid/world/layers/vegetation/tree-field-playground/captures/forest-floor-legacy.png`](maybraid/world/layers/vegetation/tree-field-playground/captures/forest-floor-legacy.png)
- [`maybraid/world/layers/vegetation/tree-field-playground/captures/forest-floor-gpu.png`](maybraid/world/layers/vegetation/tree-field-playground/captures/forest-floor-gpu.png)
- [`maybraid/world/layers/vegetation/tree-field-playground/captures/forest-far-legacy.png`](maybraid/world/layers/vegetation/tree-field-playground/captures/forest-far-legacy.png)
- [`maybraid/world/layers/vegetation/tree-field-playground/captures/forest-far-gpu.png`](maybraid/world/layers/vegetation/tree-field-playground/captures/forest-far-gpu.png)

Not measured: native Metal world-route frame times, Tracy, GPU pass duration, live re-upload counts, entity/archetype churn, peak gameplay memory, oak/rory *appearance* (they use the storybook recipe on purpose), collider parity for non-Storybook GPU groves, or shadows-on appearance. Nix was not available; tests used `cargo --locked --release`.

## Whole-world count / layer / size parity (Liam bar)

The GPU swap is the world forest present path: `GrownGrove` → `ChicoGroveHost` for every hopscotch layering. High through UltraLow kit chunks go empty. Nested `StickNode` / `FoliageNode` kit meshes therefore never spawn on the GPU arm.

Intentionally out of scope (not grove tiles): 1–5 km `CanopyBumpOut` / `MediumCanopyBumpOut` cheap-ball proxies, isolated `/show` flattened plants, typed `/show orchard` grove `LodScene` hosts, and terrain turf / POM ground cover.

`--release` `hopscotch_biome_tiles_keep_counts_layers_and_sizes` grows the center 100 m tile of one forest cell per hopscotch layering found in ±16 cells (22 of 24 `LayeringKind` values; DamasEdge and TrapThicket did not appear in that window). GPU vs legacy instance counts, stacked layer occupancy, and height mean/range stayed within 20% after raising the per-grove cap to 512 plants / 262144 descriptors and pinning recipe height/canopy to authored mids so site scales map 1:1. Tuft blades now keep authored heights.

| Layering | origin xz | layers L/G | trees L/G | bushes L/G | grass L/G | tree-h mean | bush-h mean | grass-h mean | flags |
|---|---|---:|---:|---:|---:|---:|---:|---:|---|
| AgTown | -22500,15900 | 2/2 | 384/384 | 0/0 | 249/249 | 2.20 | 0 | 0.74 | ok |
| Bush | -25700,4700 | 2/2 | 0/0 | 483/483 | 0/0 | 0 | 1.19 | 0 | ok |
| FruitPlains | -14500,7900 | 2/2 | 64/64 | 0/0 | 176/176 | 7.30 | 0 | 0.93 | ok |
| Kumulipo | -22500,-3300 | 2/2 | 55/55 | 0/0 | 0/0 | 5.82 | 0 | 0 | ok |
| LiamsSummer | -24100,-22500 | 3/3 | 0/0 | 111/111 | 3545/3545 | 0 | 1.12 | 1.63 | ok |
| LushJungle | -25700,-25700 | 1/1 | 4/4 | 0/0 | 0/0 | 5.38 | 0 | 0 | ok |
| Meadowland | -25700,-11300 | 1/1 | 1/1 | 0/0 | 0/0 | 2.51 | 0 | 0 | ok |
| MiRobles | -25700,-12900 | 2/2 | 1/1 | 61/61 | 0/0 | 3.18 | 0.99 | 0 | ok |
| OldNevada | -25700,-14500 | 2/2 | 4/4 | 0/0 | 0/0 | 5.79 | 0 | 0 | ok |
| OldSteppe | -25700,-20900 | 1/1 | 0/0 | 10/10 | 0/0 | 0 | 0.85 | 0 | ok |
| OpenTropics | -24100,-25700 | 2/2 | 1/1 | 0/0 | 308/308 | 11.40 | 0 | 0.73 | ok |
| OwlsDesert | -24100,17500 | 3/3 | 2/2 | 0/0 | 30/30 | 10.97 | 0 | 0.31 | ok |
| Riparian | -25700,-4900 | 4/4 | 5/5 | 1/1 | 17/17 | 10.30 | 0.96 | 0.64 | ok |
| Seceda | -25700,9500 | 1/1 | 0/0 | 0/0 | 255/255 | 0 | 0 | 0.74 | ok |
| SteppeDown | -25700,-1700 | 2/2 | 4/4 | 410/410 | 0/0 | 4.91 | 1.03 | 0 | ok |
| Storybook | -25700,-24100 | 3/3 | 18/18 | 47/47 | 2628/2628 | 15.45 | 1.53 | 0.74 | ok |
| SunsBarren | -25700,-3300 | 1/1 | 0/0 | 31/31 | 0/0 | 0 | 1.12 | 0 | ok |
| Taiga | -25700,-9700 | 3/3 | 40/40 | 0/0 | 578/578 | 4.48 | 0 | 0.36 | ok |
| TemperateHoly | -25700,-16100 | 1/1 | 6/6 | 0/0 | 0/0 | 16.19 | 0 | 0 | ok |
| UpperPark | -25700,-22500 | 5/5 | 3/3 | 382/382 | 235/235 | 13.86 | 1.04 | 0.74 | ok |
| Waiguo | -14500,9500 | 3/3 | 425/425 | 0/0 | 1448/1448 | 2.45 | 0 | 1.78 | ok |
| WestMaui | -25700,-8100 | 4/4 | 7/7 | 0/0 | 322/322 | 4.58 | 0 | 0.74 | ok |

GPU CPU `LodScene` is a single High band: no Mid/Low/UltraLow vegetation spawn or swap. `scene_lod_status` stays `Unchanged` when the viewer walks from the tile into the far ring; leftover non-High roots are culled. Stream in/out is still the 1 km present ring (`GROVE_PRESENT_RADIUS_M`). Legacy kits keep High/Mid/Low/UltraLow banding.

### Far-band instance counts (native-feedback coverage)

`grove_gpu_far_bands_match_legacy_instance_counts` drives the real `ChicoGroveHost` path (stacked Temperate Lower Massives + LowBush + TallGrass) and compares plants that survive GPU `lod_bucket` against legacy High (≤400 m) / Medium (≤520 m) / Low (to 1 km). Grove-owned `card_m` is unbounded so residency, not the shader, decides presence. GPU LOD still thins wood/leaves (`wood`/`leaves` below). Old cutoffs are `tree_default` / `bush_default` / `field_default` (bushes gone by ~110 m, grass by ~90 m, trees CARD after 72 m).

| Distance | Legacy T/B/G | GPU T/B/G | Old cutoff T/B/G | GPU wood / leaves |
|---|---:|---:|---:|---:|
| 50 m | 13 / 168 / 2271 | 12 / 158 / 2151 | 12 / 157 / 889 | 271 / 5979 |
| 200 m | 13 / 168 / 2271 | 13 / 168 / 2271 | 9 / 0 / 0 | 303 / 6491 |
| 450 m | 13 / 168 / 0 | 13 / 168 / 1614 | 0 / 0 / 0 | 260 / 6491 |
| 900 m | 13 / 168 / 0 | 13 / 168 / 871 | 0 / 0 / 0 | 114 / 1959 |

Woody plants stay within 20% of legacy at every band. Legacy tufts omit Medium+; GPU grass remains through 1 km with density falloff. This is a CPU-mirrored select of the real spawn path, not a live Metal census.

`hopscotch_world_walk_streams_gpu_hosts_without_kit_chunks` walks ten 1600 m cells along x=-4950 (LushJungle, SteppeDown, OldSteppe, Riparian, TemperateHoly, OwlsDesert). No leftover kit chunks, no CPU band churn, tiles stream in/out, no panics. This is the real `presenting_recipes` → `ChicoGroveHost` spawn path on flat ground, not a live HCSG terrain session.

Playground hopscotch walk captures (llvmpipe Mesa 25.2.8, 960×540, shadows off). GPU `kit_meshes=0`. Kit start/mid p95 250 ms is Bevy's max-delta clamp. Grass receipt includes 24,336 playground turf blades.

| Shot | mean ms | p95 ms | entities | kit meshes | GPU plants / bushes / grass |
|---|---:|---:|---:|---:|---|
| walk-start-legacy | 250.000 | 250.000 | 66,086 | 65,585 | — |
| walk-start-gpu | 59.707 | 60.797 | 522 | 0 | 116 / 482 / 25,334 |
| walk-mid-legacy | 250.000 | 250.000 | 66,086 | 65,585 | — |
| walk-mid-gpu | 58.241 | 60.354 | 522 | 0 | 116 / 482 / 25,334 |
| walk-end-legacy | 64.028 | 66.874 | 66,086 | 65,585 | — |
| walk-end-gpu | 13.214 | 14.583 | 522 | 0 | 116 / 482 / 25,334 |

No B0004 in the capture log. Remaining gaps: far bump-out proxies, `/show` kit plants, shadows-on admission, collider parity, two hopscotch layerings not seen in the ±16 cell window, and a live `maybraid` game walk on Metal.
