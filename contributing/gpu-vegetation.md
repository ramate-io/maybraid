# GPU vegetation

The tree-field prototype measured in [#1650](https://github.com/ramate-io/maybraid/pull/1650)
replaces part of the visual representation. It still uses the grove's CPU
streaming and LOD lifecycle. Keep these scopes separate when evaluating
the experiment or planning its successor.

## What the experiments measured

- **Canopy shadows.** The opt-in canopy-fit mode tightens nearby shrub shadows
  while keeping real sticks and leaf/frond cards excluded as casters. In the
  admitted 1.4 km prefix, CPU frame-interval p95 was 22.984 ms with stock
  proxies and 23.015 ms with fitted canopy. Both full-route controls died
  during ordinary combat, and one fixture showed an unidentified left-edge
  shadow. The earlier actual-stick arm showed worse pacing over a different,
  full-route workload. Keep these comparisons separate. [Shadow validation](https://github.com/ramate-io/maybraid/blob/39f513791474b54c6c3918aef9082e3e1fc33bb3/profiling/fitted-canopy-shadows/VALIDATION.md#canopy-fit-only).
- **Tree fields.** The tested 3×3 region replaced the visuals of 32 naturally
  placed Storybooks out of 62 woody plants across nine tiles. Other species
  retained their usual visuals. Both field arms used the same descriptors
  with shadows disabled. GPU selection reduced field CPU preparation by
  about 0.21 ms per application interval in both capture orders. End-to-end
  application pacing showed no overall win. GPU execution time, load-in, and
  LOD-transition hitches remain unmeasured by those standing windows.
  [Tree-field benchmark](https://github.com/ramate-io/maybraid/blob/3c8e6fec904365557e9a034cd870f35111627e09/profiling/world-tree-field-benchmark.md).

The [tested tree source](https://github.com/ramate-io/maybraid/commit/aacc5c07f2f26286456067e383a5384a1e26d17a)
keeps separate CPU High and Medium scene roots with per-plant sample and collision
owners. Low and UltraLow still use CPU canopy proxies. Two per-frame source
scans reconcile visible plants across warm scene roots, adding or removing a
`TreePatch` per active plant. A membership change rebuilds the aggregate
descriptor field and replaces its upload buffers. The field still resolves the
authored Storybook before generating a second skeleton for rendering. Stable
generations reuse their snapshots and buffers. The aggregate shares a bounding
box across two render presenters for wood and foliage. GPU selection chooses
among three visual bands for each draw kind. Whole-scene CPU visibility and mesh collection did not
decrease in the sampled standing windows. GPU culling work remains unmeasured.

## What to test next

1. Give each grove or tile a persistent visual descriptor owner. Keep one
   coarse CPU residency and culling decision, and let the GPU choose visual
   detail for resident descriptors. Publish and upload descriptors when the
   authored data or residency changes. Avoid rebuilding the stock visual kit
   and a second skeleton for the same field-owned plant. Preserve placement,
   species, materials, and collision ownership independently of the visual bands.
2. Compare the same descriptors and visuals with CPU and GPU selection.
   Verify the active backend, admitted counts, and actual wood and foliage
   draw paths. Use matched shadow settings and distinguish encoded indirect
   draws from measured GPU execution. A kit-versus-field comparison also
   changes the visuals, so it cannot isolate the selection cost.
3. Benchmark standing frames and the complete load, approach, cull, and
   re-entry sequence. Check visuals, support, and resident counts throughout.
   Measure frame tails, CPU preparation, entity and descriptor lifetime,
   allocation and upload work, and GPU timing where available. A standing
   result from the current prototype does not decide whether grove-owned
   visuals pay off.
