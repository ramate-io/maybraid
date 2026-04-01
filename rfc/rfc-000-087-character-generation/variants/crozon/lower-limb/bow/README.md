# Bow

**Figure (TBD):** add a reference image at `./assets/bow.jpeg` when you have a sketch.

A limb segment that is **not** straight along its axis: the volume reads like a cylinder whose cross-section **widens** toward the middle (or follows a defined width profile) while the **spine curves** between the two end caps—an arc or S-shaped sweep rather than a line extrusion.

**Semantics.** In local space, `height` is the **arc length** or the **chord-span** between end planes (pick one convention per implementation and document it). **Top** is **proximal**; **bottom** is distal. **Width** and **depth** are cross-section extents orthogonal to the local spine tangent at each sample; the **bow** lies primarily in a chosen plane (e.g. sagittal vs coronal) so rigging can align the bend predictably.

## Parameters

**API note:** **Suggestive** until implementation locks types. The spine will likely become a spline or polynomial offset from a reference line; width falloff may match the drumstick-style polynomial taper idea later.

- `height`: extent along the limb (chord or arc length—specify in code).
- `width_proximal`, `width_distal`: cross-section width at the two ends (or a single `end_width` if symmetric).
- `width_mid` (or `max_width`): cross-section width at the broadest part of the bow.
- `depth` (or `depth_proximal` / `depth_mid` / `depth_distal` if you need non-circular sections).
- `bow_amplitude`: lateral (in-bend-plane) displacement of the spine at midsegment, or a normalized scalar driving curvature.
- `bow_asymmetry`: optional skew so the widen-and-bend is stronger toward proximal or distal.

**Optional:** higher-order width curves along the arc; subtle twist of the cross-section frame along the spine.

## As a Lower Limb

Bow segments read as **soft weight** or **gesture** in the limb: good when a straight cylinder would feel too rigid. Pair with clear joint anchors at the end caps, so animation does not fight the curved rest shape.
