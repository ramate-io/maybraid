# Native grove benchmark on f192490

The captures below test f1924904758d8c8768c3edc648444aa456939210. Newer PR revisions require a separate native retest. [Retest request](https://github.com/ramate-io/maybraid/pull/1652#issuecomment-7083088640)

GPU fields worsened application pacing in both route orders. Full-route p95 increased by 39.7% and 41.8%. Peak sampled process footprint fell in both pairs while peak RSS rose. In a separate five-second standing pair, Metal recorded less process GPU busy time in GPU mode while simultaneous application intervals grew longer. These results compare the two realized world presentations end to end.

The main captures used the same release binary and ~3 km input, native Metal at fullscreen 1920×1080, render scale 1, verified Immediate mode and shadows off. All four completed 15 waypoints with one player at 100 HP and passed foreground checks. Binary SHA-256 is 1c91703179836e3b5beb820c8ca6a6504eda1c0fe75f395ccba4a72002c17e2f, built with bevy/trace_tracy and debug-commands. Cargo.lock SHA-256 is f8895e2c3daaa0e8078b26882d22d23061a360b8cf9f2c848fbe748b6df7b47d. [Tested source](https://github.com/ramate-io/maybraid/commit/f1924904758d8c8768c3edc648444aa456939210)

## Route timing and memory

| Order | Mode | Observed km | App gaps | p95 ms | p99 ms | Max ms | Support s | Peak footprint MiB | Peak RSS MiB |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| First | Legacy | 3.005 | 75,244 | 11.168 | 12.509 | 508.112 | 6.069 | 5,474.99 | 4,656.78 |
| First | GPU | 2.995 | 57,151 | 15.606 | 17.586 | 681.163 | 6.045 | 5,271.82 | 4,884.28 |
| Reversed | GPU | 2.995 | 56,675 | 15.555 | 17.409 | 127.317 | 6.308 | 5,325.94 | 4,934.19 |
| Reversed | Legacy | 3.003 | 76,156 | 10.971 | 12.404 | 154.639 | 6.065 | 5,515.44 | 4,686.30 |

Application intervals are consecutive Tracy main-app starts wholly inside the fixed phase marks. All admitted gaps are retained, including outliers. Loading CPU coverage begins at the first traced main-app entry. Launch-to-ready support timing uses independent process and readiness receipts. Approach, anchor stand, departure, re-entry, and returned stand had higher p95 in GPU mode. Far stand went from 8.866/8.771 ms legacy to 8.320/8.230 ms GPU, with 34 active NPC bodies legacy and about 12–13 GPU. At the anchor stand the corresponding counts were 24→5 and 16→1. Individual maximum intervals and intervals above 100 ms changed direction between orders. Readiness measures process launch to supported gameplay. Footprint and RSS are sampled maxima from roughly one-second exact-PID samples.

CPU render wrapper p95 rose 10.455→14.733 ms and 10.251→14.664 ms. CPU visibility changed from 0.1123→0.1049 ms per application interval in the first pair and 0.1122→0.1058 ms in the reverse pair. LOD refresh region production and cull drain rose slightly, binned instance-buffer writing fell, and the traced deferred scene-root merge command scope was about 0.00010 ms per interval throughout. GPU r2 measured 0.47251 ms per interval in tree buffer preparation and 0.09884 ms in CPU recording of GPU selection commands. The GPU r1 broad tree scope exceeded its export cap. CPU scope wall times can overlap.

Both GPU routes reproduced 466, 548, 295 and 548 tree owners at load, anchor, far and return. Each recorded 1,050 publications, 1,050 descriptor uploads and 502 retirements, with 124,771,184 uploaded tree descriptor bytes per run and 41,192,800 on re-entry. The standing phases had zero tree descriptor republish, retirement or re-upload. Tree GPU setup still writes per-owner uniforms and records selection dispatches for visible instances. [Persistent tree ownership](https://github.com/ramate-io/maybraid/blob/f1924904758d8c8768c3edc648444aa456939210/maybraid/world/layers/vegetation/tree-field/src/plugin.rs#L118-L169) · [Uniform write](https://github.com/ramate-io/maybraid/blob/f1924904758d8c8768c3edc648444aa456939210/maybraid/world/layers/vegetation/tree-field/src/gpu.rs#L828-L855) · [Selection dispatch](https://github.com/ramate-io/maybraid/blob/f1924904758d8c8768c3edc648444aa456939210/maybraid/world/layers/vegetation/tree-field/src/gpu.rs#L942-L1013)

## Matched Metal and simultaneous Tracy

| Measure | Legacy | GPU |
|---|---:|---:|
| Exact-process Active GPU union over 5,000 ms | 4,766.994 ms | 3,015.092 ms |
| Busy fraction | 95.34% | 60.30% |
| Main-app gaps observed | 739 | 354 |
| App p95 | 8.054 ms | 15.617 ms |
| Main CPU wrapper p95 | 7.332 ms | 9.072 ms |
| Render CPU wrapper p95 | 5.010 ms | 14.591 ms |
| RenderExtractApp bridge p95 | 0.836 ms | 7.840 ms |
| prepare_windows CPU scope p95 | 0.227 ms | 0.117 ms |
| prepare_windows CPU scope p99 | 0.949 ms | 1.641 ms |

Both dual-observer runs used the same standing input. Each window was fixed at 5–10 s after the standing marker before inspection. The Metal TOC and game UTC marker established each window. Auxiliary Tracy used the same phase marker and offsets. Tracy-to-UTC phase spans differed by −0.439 and −0.417 ms. This checks span agreement. Absolute marker delivery and timestamp mapping retain uncertainty.

Exact PIDs accounted for 31,865 legacy and 88,975 GPU-mode Active intervals. GPU mode excluded 12 unknown-PID intervals with 1.028 ms of union. Legacy had 4,992.962 ms in Maximum state. GPU mode had 3,623.000 ms in Medium and 1,211.418 ms in Minimum. GPU selection labels occupied 417.065 ms for tree, 24.369 ms for bush and 19.461 ms for grass. Label unions can overlap. This is window-level GPU activity with different application counts, states, and realized actors. Legacy had 3 active NPC bodies and brains, and GPU mode had 14. Per-frame GPU attribution was unavailable across the pair.

The RenderExtractApp bridge timing includes waiting for the previous rendering world, running extraction, and returning that world to the render thread. Bush and grass clone owned field vectors into a main-world extracted resource and clone them again through the extraction derive. The tree owner map shares reference-counted Arc snapshots. Clone costs would require a dedicated timing scope. The exact prepare_windows system had 740 and 354 invocations in these windows. Its means were 0.089 and 0.096 ms, so p95 and p99 moved in different directions. [Bevy bridge](https://github.com/ramate-io/bevy/blob/dfbb7d045b15879232dde2efc9bc66f499413ef6/crates/bevy_render/src/pipelined_rendering.rs#L217-L238) · [Sub-app wrapper](https://github.com/ramate-io/bevy/blob/dfbb7d045b15879232dde2efc9bc66f499413ef6/crates/bevy_app/src/sub_app.rs#L575-L587) · [Bush snapshot](https://github.com/ramate-io/maybraid/blob/f1924904758d8c8768c3edc648444aa456939210/maybraid/world/layers/vegetation/bush-field/src/plugin.rs#L120-L144) · [Grass snapshot](https://github.com/ramate-io/maybraid/blob/f1924904758d8c8768c3edc648444aa456939210/maybraid/world/layers/vegetation/grass-field/src/plugin.rs#L120-L144) · [Extraction derive](https://github.com/ramate-io/bevy/blob/dfbb7d045b15879232dde2efc9bc66f499413ef6/crates/bevy_extract/macros/src/extract_resource.rs#L34-L41)

## Content and checkpoints

The field changes geometry, species presentation, density and CPU LOD. Claimed grove chunks emit an empty legacy primitive, bypassing metadata used by legacy stick collider producers. Bush and grass aggregate capacity and selection can drop producer-admitted instances. The primary logs measure producer admissions and tree ownership. [Field recipes](https://github.com/ramate-io/maybraid/blob/f1924904758d8c8768c3edc648444aa456939210/maybraid/world/layers/vegetation/chico/src/tree_field/visuals.rs#L72-L138) · [Chunk path](https://github.com/ramate-io/maybraid/blob/f1924904758d8c8768c3edc648444aa456939210/maybraid/world/layers/vegetation/chico/src/host.rs#L160-L174) · [Bush cap](https://github.com/ramate-io/maybraid/blob/f1924904758d8c8768c3edc648444aa456939210/maybraid/world/layers/vegetation/bush-field/src/field.rs#L31-L87) · [Grass cap](https://github.com/ramate-io/maybraid/blob/f1924904758d8c8768c3edc648444aa456939210/maybraid/world/layers/vegetation/grass-field/src/field.rs#L27-L77)

The separate checkpoint binary is 88080cb9bbb44fa1feb2c06503c92035ad21b740ad4f358061e5815709c5f40a, built from f192490 with debug-commands and memory-census. It applies the observation overlay SHA-256 9b9c72df97127d8331c9bfc83f92ef7423cca356c1d65e39f2aae35606c2759f followed by compatibility patch SHA-256 5186335614ceea51c0121e8a6e20190497f64fae96092ac3e2a92ee71f44b086. The combined staged diff SHA-256 is afd744ca18ef48d74d795bc340508114bc33f6b6eab441927263054c8b1b59d6. The first observer build failed because its Bevy On<Add,T>/On<Remove,T> signatures differed from the pinned fork. The two-line patch corrected the observer signatures in this diagnostic build. Both observer routes completed all 15 waypoints and four ordered checkpoints. Collider, stick-marker, and tree-owner counts include stored Disabled entities. Bush and grass inputs count enabled source patches. The aggregate counts are CPU descriptor contents.

| Checkpoint | Live entities Legacy / GPU | Retained archetypes Legacy / GPU | Collider components Legacy / GPU | Stick compounds Legacy / GPU |
|---|---:|---:|---:|---:|
| Loaded | 41,945 / 34,901 | 1,887 / 1,831 | 907 / 272 | 635 / 0 |
| Anchor | 64,188 / 47,717 | 2,126 / 2,059 | 2,141 / 535 | 1,620 / 0 |
| Far | 62,360 / 35,919 | 2,326 / 2,295 | 1,499 / 312 | 1,119 / 0 |
| Returned | 71,326 / 47,536 | 2,427 / 2,361 | 2,175 / 518 | 1,657 / 0 |

GPU mode had zero stored stick producer markers at all four checkpoints. At the anchor, 4,994 enabled bush samples yielded 3,125 aggregate plants and 135,713 enabled grass samples yielded the 131,072-blade cap. At the far stand, 161,192 enabled grass samples yielded the same cap. These counts describe storage. Per-frame GPU survival, compound shape totals, and contact parity remain unmeasured.

The returned checkpoint had 15,649 additions and 6,853 removals of WorldAssetRoot in legacy, compared with 3,137 and 1,924 in GPU mode. Mesh3d had 25,406 and 10,195 in legacy compared with 4,842 and 2,622 in GPU mode. Counts are cumulative since observer installation and reflect named component memberships. Retained archetypes include empty archetypes. Full entity spawn/despawn and archetype migration require dedicated counters. [Census metrics](https://github.com/ramate-io/maybraid/releases/download/v0.1.5-traces/f192490_20261122_m4pro-macmini_grove-checkpoint-census.json)

## Failed controls and artifacts

The CPU-requested smoke reported CpuSelect at startup but both actual tree draw kinds encoded GpuDriven. The run did not establish a CPU-versus-GPU selection comparison. A screenshot smoke failed on an offscreen bush target-format mismatch. Failed Metal attempts include desktop lock, a DTX service stall, and recording finalization timeouts. The first legacy census stopped at a stuck waypoint and preserved two incomplete checkpoints. The exact-route retry completed all four. The completed 25 s legacy Metal recording provided one-arm context. The shorter matched pair supplies the comparative GPU result above.

The v0.1.5 Traces package contains four walking Tracy files, two matched Metal bundles, and the two simultaneous Tracy files with cross-linked same-stem companions. The provenance archive includes selected manifests, route/runner scripts, phase and scope summaries, exact time alignment, census observer patches and checkpoint metrics. Failed artifacts stay in the local run ledger. [Traces release](https://github.com/ramate-io/maybraid/releases/tag/v0.1.5-traces) · [Benchmarked capture provenance](https://github.com/ramate-io/maybraid/releases/download/v0.1.5-traces/f192490_20261122_m4pro-macmini_grove-benchmark-provenance.zip)
