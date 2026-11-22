# Grove-owned visual investigation

This continues [#1647](https://github.com/ramate-io/maybraid/pull/1647) and the partial replacement in [#1650](https://github.com/ramate-io/maybraid/pull/1650), following [#1651's investigation guides](https://github.com/ramate-io/maybraid/pull/1651). Implementation and native validation are in progress; no performance improvement is established by this document.

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

`profiling/grove-lifecycle.py prepare PATH` emits a proposed 3 km ordinary walking route: loading at `(-4950,3750)`, 600 m approach, standing, 1.2 km departure, then 1.2 km return and standing. Scout the route before admitting paired results. Retain failures, deaths, unsupported motion and incomplete phase records. Report launch-to-supported readiness separately from script windows. The selected area covers only part of the route.

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
