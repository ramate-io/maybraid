# Drum Prong

**Figure (TBD):** add a reference image at `./assets/drum-prong.jpeg` when you have a sketch.

Similar to [Drumstick](../drumstick/README.md), but the **distal** end **splits** into **two** curving cylindrical prongs instead of ending as a single shaft. The proximal portion can reuse the same bulb-and-shaft vocabulary as the drumstick; the fork is the distinguishing feature.

**Semantics.** Align with [Drumstick](../drumstick/README.md): `height` along the limb, **top** proximal, **bottom** distal. The **split plane** contains the two prong axes as they leave the shared trunk; **prong** curvature and separation are defined relative to that plane and the distal direction.

> [!NOTE]
> A practical construction is often **CSG-style**: start from a [Drumstick](../drumstick/README.md) volume and subtract a **cone-like** (or wedge) negative at the distal end to carve the cleft, then **round or bevel** the prong tips. Document the exact ops when the pipeline exists.

## Parameters

**API note:** **Suggestive.** Fork geometry may move to spline-based prong centerlines and explicit tip radii; many fields can mirror drumstick until then.

- `height`: total extent along the limb axis (trunk + prongs along their outer envelope, or define consistently with your bounding convention).
- `depth`, `width`: cross-section semantics as on the drumstick trunk; prongs may inherit scaled versions.
- Drumstick-aligned placeholders (reuse or subset): `drop`, `bulge_to_shaft_ratio`, `bulge_width_taper`, `offset`—see [Drumstick](../drumstick/README.md).
- `split_start`: axial distance from **proximal** (or from distal—pick one and document) where the single trunk ends and the fork begins.
- `prong_separation_angle` (or `prong_tip_spacing`): how far apart the prong tips are at the distal end.
- `prong_curve`: placeholder scalar for how strongly each prong bows (polynomial/spline later).
- `prong_radius`: cross-section radius of each cylindrical prong (or `prong_width` / `prong_depth` if elliptical).

**Optional:** asymmetric prong length or radius; geodesic or noise detail on the hull.

## As a Lower Limb

The Drum Prong is good for exotic creatures. It combines a familiar outline with forlorn detail. Place skeleton **distal** targets at each prong tip (or a single foot socket with a rule for averaging) depending on your rig.
