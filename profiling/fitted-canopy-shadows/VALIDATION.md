# Fitted canopy and actual-stick shadows: validation

Keep [#1649](https://github.com/ramate-io/maybraid/pull/1649) draft and opt-in. Fitting visibly reduces oversized nearby bush shadows, and actual typed stick geometry becomes caster-eligible without enabling leaf/frond cards. However, the approximately 2.4 km walks show a material pacing cost: p95 rises from 30.015 ms with existing proxies to 35.755 ms fitted at 2048. Fitted 1024 remains at 35.525 ms and softens shadow edges. Full actual-stick casting is not ready as the default.

The separate receiving fix is [#1632](https://github.com/ramate-io/maybraid/pull/1632). All arms here have receiving ON. This experiment preserves Proxy as the default; `MAYBRAID_CANOPY_SHADOWS=fitted` opts in. The independent `MAYBRAID_EXPERIMENT_SHADOW_MAP_SIZE=1024` changes High resolution alone, preserving four cascades and 150 m range.

The later `canopy-fit` source arm is described in [README](README.md); none of the measurements below include it.

## Short comparison

One ordered, approximately 15-second proxy-2048 → fitted-2048 → fitted-1024 comparison:

| Arm | Mean ms | p50 / p95 / p99 ms | Maximum ms | >20 ms |
|---|---:|---|---:|---:|
| Proxy 2048 | 11.350 | 11.326 / 11.946 / 12.328 | 18.516 | 0 |
| Fitted 2048 | 11.476 | 11.480 / 12.075 / 12.617 | 13.161 | 0 |
| Fitted 1024 | 11.483 | 11.475 / 12.114 / 12.544 | 18.765 | 0 |

Fitted 2048 mean/p95 were about +0.126/+0.130 ms versus Proxy. The 1024 comparison shows no obvious pacing recovery in this small scene. Nearest-rank short-window tails and median p50 use 1,320/1,306/1,305 application-start gaps. The secondary >1 ms pipeline-queue overlap exclusion removes zero gaps.

## Long walks

The same route completed 2,404.686/2,409.754/2,411.734 m supported cumulative XZ through checkpoint 118. These distances are cumulative travel, not displacement.

| Arm | Tracy-marked walking seconds | Gaps | p50 / p95 / p99 ms | Maximum ms | >33.33 / >50 / >100 ms |
|---|---:|---:|---|---:|---|
| Proxy 2048 | 502.618 | 27,171 | 17.527 / 30.015 / 32.775 | 500.732 | 195 / 13 / 5 |
| Fitted 2048 | 521.232 | 24,075 | 20.160 / 35.755 / 37.910 | 133.050 | 2,129 / 18 / 2 |
| Fitted 1024, closed walk only | 520.618 | 24,206 | 20.130 / 35.525 / 37.934 | 164.677 | 1,947 / 17 / 1 |

Fitted p95 is +19.1% at 2048 and +18.4% at 1024 relative to Proxy in these observations. A common three-arm route-segment observer/detour mask preserves the pattern: p95 30.038/35.782/35.562 ms, retaining 26,928/23,826/23,945 gaps. Long-walk percentile interpolation is linear. These are CPU-wall main-application entry gaps, not measured GPU frames.

For the 2048 pair, render-app p95 rose 28.955→34.664 ms, main-app 21.090→22.814 ms and extract 10.440→14.398 ms. Enclosing wall-time scopes do not identify the internal cause. Proxy's largest 500.7/407.8 ms gaps overlap long pipeline-queue scopes. Both fitted runs retain main-app-dominated >100 ms gaps around checkpoint 45–47; lower maxima do not demonstrate a general hitch improvement.

The 1024 walk closed healthy, then a Kappler Raider killed the player 40.89 seconds later during the ending standing window. That lifecycle failed and remains failed. Only its initial phase and closed healthy walk are admitted; no ending-standing or whole-process comparison is made for 1024.

## Visual and caster observations

Realized spheres total 5,367/5,331/5,331 across the short trio, versus camera-visible mesh counts 1,564/1,501/1,537. Cascade counts are 40/74/201/2,129, 44/84/288/2,539 and 44/84/288/2,541. Four nearby actual typed sticks are loaded and cascade-listed in fitted mode; leaf/frond recipe caster candidates remain zero. CPU eligibility and cascade membership do not prove every GPU submission.

Foreground bush shadows shrink from oversized faceted disks to tighter ovals. The nearest sphere scale changes from approximately `(1.102, 1.102, 1.102)` to `(0.567, 0.243, 0.559)` m. One ellipsoid deliberately fills crown gaps. At 1024 the placement remains similar but actor/weapon and large-shadow edges soften. Fine branch silhouettes are not isolated from canopy occlusion; an additional large shadow at the left edge has an unidentified offscreen owner.

Long-walk checkpoint 26 also shows a more localized shrub shadow; checkpoint 105 is only modestly tighter/similar. Checkpoint 65 has different nearby framing and silhouettes. Recorded player XZ deltas at these checkpoints are 0.139/2.306/0.038 m with equal yaw/pitch, not exact screenshot camera transforms. Stills do not establish temporal aliasing, motion or LOD-transition stability.

## Sampled process memory and workload limits

| Window | Proxy 2048 footprint / RSS GiB | Fitted 2048 footprint / RSS GiB | Fitted 1024 footprint / RSS GiB |
|---|---:|---:|---:|
| Initial median | 6.230 / 5.347 | 6.519 / 5.351 | 6.509 / 5.360 |
| Walking median | 8.858 / 7.314 | 8.537 / 7.218 | 8.703 / 7.277 |
| End-standing median | 10.199 / 7.933 | 9.824 / 8.087 | Unavailable |

Walking phases contain 101/104/104 samples at approximately five-second cadence, all bound to exact PID/start identity. The two complete processes sampled footprint maxima of 10.209/9.825 GiB but RSS maxima of 7.948/8.089 GiB. The 1024 admitted initial-to-walk epoch sampled 9.780 GiB footprint and 8.074 GiB RSS maximum; these are not whole-process maxima. Different starting memory, higher fitted RSS and unequal content history prevent a causal memory-saving claim. Instantaneous peaks, asset ownership and GPU residency are not measured.

Both complete 2048 runs grew substantially: initial-to-end footprint medians +3.969/+3.305 GiB and RSS +2.587/+2.736 GiB. Moving revisit-52/104 footprint rose by 615/947 MiB; these were not settled origin returns. Ending positions remained approximately 691 m from the initial position. Standing windows drifted approximately 3–5 m and are labeled nonstationary.

All admitted walks kept the same living actor within each run at HP100 and original gravity, with no observed typed player shots or applied damage. That does not inventory all NPC activity. In the 2048 pair, checkpoint XZ separation has median 0.563 m and maximum 14.836 m; two checkpoints exceed 5 m. Change-driven enabled-host/body maxima differ (69/72 versus 62/70). Both record 153 typed chunk additions, which are additions rather than resident content counts. The ordered runs, differing realized workload and 4.03%/2.34% short-view mesh variation limit causal precision. Actual GPU execution time is unmeasured; CPU wrappers cannot establish a GPU bottleneck or available GPU headroom.

## Source verification and preserved failures

Native source `d3ef115d20f4c4af186dc5c9a6a6d61f2f087875` used published Bevy `dfbb7d0`, one immutable executable (`9f476e8e…`) and the same read-only diagnostic observer. Features are `debug-commands,bevy/trace_tracy`; fullscreen 1920×1080 Metal Immediate, noon lighting and High four-cascade/150 m settings were verified. Independently recomputed CSV statistics, three-arm masks and all 27 export hashes pass; no raw traces were uploaded.

[Exact-head Linux Cargo Test](https://github.com/ramate-io/maybraid/actions/runs/44944689190/job/136969298923) passed `cargo test --release --locked`, including seven new focused fixtures: mode/card policy, authored axes, bush fit, blade envelope, many-puffs-to-one/legacy eligibility, typed-stick/ancestor opt-out and High-only 1024. Script smoke also passed all five scenarios.

The initial per-puff prototype produced 66,939 versus 5,367 spheres (12.47×) and failed admission before timing. The corrected source preserves legacy eligibility and emits at most one marker per eligible normal plant, none for invalid/empty canopy. A wider stationary fixture failed its nearby-stick camera-membership gate before screenshot/timing. The first fitted walk stopped at an obstructed connector after 514.728 m and is excluded; the unchanged-route retry passed. No changed collider generation or visual placement was found in source after the shared Low-geometry correction, but the obstruction's cause is not established.

Generic coarse-site visual placement is preserved; fitted mode still excludes bark-colored coarse-column shadow candidates. Independent `NotShadowCaster` opt-outs stay authoritative; optional Full is unchanged. Fitting traverses authored High data once per scene/chunk emission, not per frame; its generation cost has not been separately attributed.

The next useful experiment is canopy fitting alone, then near-only actual sticks with canopy proxies farther away if profiling supports it. Neither was implemented in the captured revision. The visual improvement is real in the recorded views; the measured long-walk cost keeps this candidate opt-in.
