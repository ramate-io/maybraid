# Drum prong

**Figure (TBD):** add a reference image at `./assets/drum-prong.jpeg` when you have a sketch.

Like [Drumstick](../drumstick/README.md), but the **distal** end (along the primary axis) **splits** into **two** curving cylindrical **prongs** instead of a single shaft. The proximal portion reuses the drumstick bulge-and-shaft vocabulary; the fork is the distinguishing feature.

**Semantics.** Align with [Drumstick](../drumstick/README.md): `height` along the segment, **top** / **bottom** at the two ends. The **split plane** contains the two prong axes as they leave the shared trunk; prong curvature and separation are defined in that plane and along the distal direction.

> [!NOTE]
> A practical build is often **CSG-style**: start from a [Drumstick](../drumstick/README.md) volume and subtract a **cone-like** (or wedge) negative at the distal end to carve the cleft, then **round or bevel** the prong tips. Document exact ops when the pipeline exists.

## Parameters

**API note:** **Suggestive.** Fork geometry may move to spline-based prong centerlines and explicit tip radii; many fields can mirror drumstick until then.

- `height`: total extent along the long axis (trunk + prongs—define bounding convention in code).
- `depth`, `width`: cross-section semantics as on the drumstick trunk; prongs may use scaled copies.
- Drumstick-aligned placeholders (reuse or subset): `drop`, `bulge_to_shaft_ratio`, `bulge_width_taper`, `offset`—see [Drumstick](../drumstick/README.md).
- `split_start`: axial position where the single trunk ends and the fork begins (measure from **top** or **bottom**—document once).
- `prong_separation_angle` (or `prong_tip_spacing`): separation of prong tips at the distal end.
- `prong_curve`: placeholder for how strongly each prong bows (polynomial/spline later).
- `prong_radius`: cross-section radius of each prong (or `prong_width` / `prong_depth` if elliptical).

**Optional:** asymmetric prong length or radius; geodesic or noise on the hull.

## Crozon feature uses

- [Lower limb](../../lower-limb/drum-prong/README.md)
