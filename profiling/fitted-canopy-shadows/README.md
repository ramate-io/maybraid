# Fitted canopy and actual stick shadow experiment

This opt-in experiment stacks on the receiving correction in [#1632](https://github.com/ramate-io/maybraid/pull/1632). The existing `proxy` default, `full`, and `off` selections remain available. This is source staging: Rust compilation, new fixtures, image acceptance and overhead measurements are pending.

`MAYBRAID_CANOPY_SHADOWS=fitted` selects actual `StickNode` trunk/branch geometry plus shadow-only canopy balls. Actual single and merged stick scene roots carry typed provenance. Only that provenance bypasses the vegetation stick exclusion; independent `NotShadowCaster` ancestors remain authoritative. Bark-colored `FoliageNode` impostor columns do not gain casting merely from their material. The fitted path does not enable real leaf or frond cards, whose wind/discard silhouette is not reproduced by their current depth shaders.

Canopy balls use authored High foliage placements and composed collection bounds, not the whole-plant `StructuralLod.tree_radius`. Separate ball nodes retain deliberate puffs; each authored collection becomes one approximate canopy envelope, and standalone blade nodes aggregate into one envelope per plant. A frond envelope approximates the authored blade bounds, not exact cutouts or wind. Empty foliage collections emit no phantom unit sphere. The generic Low canopy site uses the canopy-only union; existing species-specific crown/column forms are retained. Building casters and receiving opt-outs are unchanged.

High/Medium fitted scene generation reads authored High foliage and fits collections on construction, not per frame. This adds generation work and can change caster count. The native admission request must report actual realized proxy/stick counts and exclusions; inspect representative bush, broadleaf and palm/conifer geometry before performance acceptance. No cache, new shadow LOD policy, or automatic quality change is included.

## Independent map-resolution control

`MAYBRAID_EXPERIMENT_SHADOW_MAP_SIZE=1024` changes only the High shadow-map resolution. It keeps four cascades and the normal High 150 m range. It does not select the Low preset (512 pixels / 60 m). Unset is the existing 2048 High setting. The override accepts only the explicit `1024` token and has no effect on Low or Off.

## Validation to run

Use one immutable binary with the published locked Bevy graph, existing `debug-commands,bevy/trace_tracy` diagnostic feature set and ordinary dirty uploads. Keep receiving ON, scene/camera/sun, four cascades, fullscreen Metal and VSync OFF equal. Compare:

| Arm | Canopy mode | High map size |
|---|---|---:|
| Baseline | proxy | 2048 |
| Candidate | fitted | 2048 |
| Resolution control | fitted | 1024 |

First run the focused default/fit/composition/count, actual-stick/material/ancestor-exclusion, and map-resolution/cascade fixtures. Tests are staged, not run. Reuse the exact native graph for fixture linking where possible; do not silently trigger a cold full-workspace test graph.

Require actual loaded typed trunk/branch caster evidence, excluded visual cards, visible canopy/ground silhouettes, and appropriate small-bush proportions. Inspect depth/culling, acne, peter-panning and cascade/LOD transitions. Compare CPU pacing and tails, real GPU timing in separately bounded Metal captures, and process footprint/RSS. Counts and CPU RenderApp spans cannot establish GPU time or memory savings. A source-only result does not justify default enablement.
