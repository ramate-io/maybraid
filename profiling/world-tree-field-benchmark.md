# Natural Storybook tree-field benchmark

On native Apple M4 Pro Metal, the GPU field rendered the same natural Storybook placements as the CPU field. GPU selection reduced the field's CPU preparation work by about **0.21 ms per application interval** in both capture orders. Application frame gaps were longer for the GPU arm in both orders, with a smaller difference in the reverse order. Both field arms used more median process footprint than the kits. This result supports keeping the experiment opt-in.

## Scene and workload

The measured source was `aacc5c07`, with the published Bevy fork at `dfbb7d0`. One release executable ran the kit, CPU field and GPU field arms. Each 15-second standing window used the same natural southwest view, nine 100 m grove tiles and exact placement digests. The selected neighborhood had 32 Storybooks among 62 woody plants. Both field modes held 32 leafy plants, 716 branches and 15,820 leaves through their windows. All six arms ran fullscreen Metal with applied Immediate mode, shadows set to Off and zero observed traced shadow-map draw spans. [Measured source](https://github.com/ramate-io/maybraid/commit/aacc5c07f2f26286456067e383a5384a1e26d17a).

The ordinary grounded player stayed at 100 HP with zero movement input. Slope drift was 0.50–0.53 m. Within each order the field-arm cameras matched the fresh kit control within 4.3 cm before and after timing. The GPU arm dispatched compute selection, encoded indirect wood and foliage draws, and visibly rendered both. CPU and GPU field images matched visually. The field replaced the kits with larger, brighter foliage and different branch geometry. The selected placements, surrounding foliage and terrain remained identifiable.

## Result

Values are consecutive application-entry gaps in milliseconds. All six windows had zero gaps above 20 ms.

| Capture order | Arm | Mean | p95 | p99 |
|---|---|---:|---:|---:|
| Kit → CPU → GPU | Kit | 7.094 | 8.451 | 8.707 |
| Kit → CPU → GPU | CPU | 7.077 | 8.483 | 8.888 |
| Kit → CPU → GPU | GPU | 7.497 | 8.533 | 9.316 |
| GPU → CPU → Kit | Kit | 7.027 | 8.436 | 8.665 |
| GPU → CPU → Kit | CPU | 7.140 | 8.502 | 8.798 |
| GPU → CPU → Kit | GPU | 7.178 | 8.625 | 8.917 |

GPU means were 5.7% and 2.1% higher than kits, and 5.9% and 0.5% higher than the same-descriptor CPU field. Render CPU wrapper means for kit, CPU and GPU were 3.847 / 4.620 / 6.453 ms in the first order and 3.831 / 4.714 / 4.229 ms in reverse. The GPU-versus-CPU wrapper difference changed direction with order.

CPU field preparation was 0.234 and 0.233 ms per application interval. GPU field preparation was 0.026 and 0.022 ms, with an additional 0.119 and 0.116 ms in related compute-system, command-application and command-buffer finishing spans. Standard preprocess bind-group and binned upload spans were higher for the field arms in both orders. These CPU spans can overlap, so the report treats each category separately. GPU execution duration was unmeasured.

The median sampled process footprint for CPU and GPU was +52.3 and +26.5 MiB against kits in the first order and +11.9 and +7.9 MiB in reverse. Median RSS changed by −1.4 and +5.8 MiB first and +8.0 and +9.8 MiB in reverse. Each arm contributed three exact-process samples at five-second intervals. Short-lived peaks and GPU residency remain unmeasured.

The two orders cover one 32-tree natural view with shadow maps disabled. Kit-to-field includes artistic and geometry substitution. CPU-to-GPU holds the field descriptors fixed and compares the selection and submission paths. The field has no matching shadow caster or receiver path. This result establishes working native selection and a local preparation saving in this scene. Broader pacing and memory benefits remain unproven.

The measured commit's Cargo Check, Cargo Test, formatting and Script Smoke completed successfully. The Cargo Test log includes the exact one/nine-tile selection, warm-overlap deduplication, separate visual presenters and shared descriptor snapshot fixtures. [Cargo Test](https://github.com/ramate-io/maybraid/actions/runs/44961685246).
