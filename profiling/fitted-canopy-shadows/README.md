# Fitted canopy and actual stick shadow experiment

This opt-in experiment stacks on the receiving correction in [#1632](https://github.com/ramate-io/maybraid/pull/1632). The existing `proxy` default, `full`, and `off` selections remain available. Native and CI validation found tighter nearby bush shadows but a material long-walk pacing cost at both 2048 and 1024. Keep the experiment opt-in; see [measured validation and limitations](VALIDATION.md).

`MAYBRAID_CANOPY_SHADOWS=fitted` selects actual `StickNode` trunk/branch geometry plus shadow-only canopy balls. Actual single and merged stick scene roots carry typed provenance. Only that provenance bypasses the vegetation stick exclusion; independent `NotShadowCaster` ancestors remain authoritative. Bark-colored `FoliageNode` impostor columns do not gain casting merely from their material. The fitted path does not enable real leaf or frond cards, whose wind/discard silhouette is not reproduced by their current depth shaders.

Canopy balls use authored High foliage placements and composed collection bounds, not the whole-plant `StructuralLod.tree_radius`. All valid authored foliage bounds stream into one canopy envelope per legacy-eligible individual plant, rather than treating dense ball samples as separate crowns. This deliberately fills crown gaps; it does not preserve independent puffs or exact leaf silhouettes. A frond envelope approximates the authored blade bounds, not exact cutouts or wind. Empty foliage collections emit no phantom unit sphere. Generic grove Low/UltraLow visual geometry and coarse site transforms remain unchanged; fitted mode excludes bark-colored coarse column shadow candidates. Fitted shadow-only volumes apply on the normal individual-plant path, including species that retain individual plants at Low. Existing species-specific crown/column forms are retained. Building casters and receiving opt-outs are unchanged.

Individual-plant fitted scene generation reads authored High foliage and fits collections on construction, not per frame. This adds generation work and can change caster count. The native admission request must report actual realized proxy/stick counts and exclusions; inspect representative bush, broadleaf and palm/conifer geometry before performance acceptance. No cache, new shadow LOD policy, or automatic quality change is included.

## Independent map-resolution control

`MAYBRAID_EXPERIMENT_SHADOW_MAP_SIZE=1024` changes only the High shadow-map resolution. It keeps four cascades and the normal High 150 m range. It does not select the Low preset (512 pixels / 60 m). Unset is the existing 2048 High setting. The override accepts only the explicit `1024` token and has no effect on Low or Off.

## Validation protocol and results

Use one immutable binary with the published locked Bevy graph, existing `debug-commands,bevy/trace_tracy` diagnostic feature set and ordinary dirty uploads. Keep receiving ON, scene/camera/sun, four cascades, fullscreen Metal and VSync OFF equal. Compare:

| Arm | Canopy mode | High map size |
|---|---|---:|
| Baseline | proxy | 2048 |
| Candidate | fitted | 2048 |
| Resolution control | fitted | 1024 |

Exact-head Linux release tests passed all seven new focused default/fit/composition/count, actual-stick/material/ancestor-exclusion, and map-resolution/cascade fixtures. The [validation report](VALIDATION.md) records native images, short comparisons, long walks, rejected attempts and the partial 1024 lifecycle.

Require actual loaded typed trunk/branch caster evidence, excluded visual cards, visible canopy/ground silhouettes, and appropriate small-bush proportions. Inspect depth/culling, acne, peter-panning and cascade/LOD transitions. Compare CPU pacing and tails, real GPU timing in separately bounded Metal captures, and process footprint/RSS. Counts and CPU RenderApp spans cannot establish GPU time or memory savings. Current measurements do not justify default enablement.

The first native build was deliberately stopped before completion after finding that the initial generic Low-site branch also changed visible geometry. That branch and its unused union helper were removed; the generic grove composition file is byte-identical to the receiver-only baseline. No performance result is attributed to the stopped build.

The first fitted admission was rejected before timing: 66,939 realized spheres versus 5,367 in the control (12.47×), with unequal visible mesh counts. Ball nodes can be dense leaf samples, not canopy masses. The bounded correction restores the same structural-LOD eligibility as the legacy proxy and emits at most one fitted marker per eligible individual plant (none for empty/invalid foliage). No timing or GPU conclusion is drawn from that rejected attempt. Generic coarse site placements remain unchanged, but fitted mode excludes bark-colored coarse column proxy casters; do not interpret that as all Low shadow behavior being unchanged.
