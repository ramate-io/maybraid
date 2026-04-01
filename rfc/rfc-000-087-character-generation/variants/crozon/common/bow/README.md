# Bow

**Figure (TBD):** add a reference image at `./assets/bow.jpeg` when you have a sketch.

An elongated volume whose **spine curves** between two end caps (arc or S-shaped sweep), with a cross-section **width** profile along the arc—wider midsegment is typical. Not a straight extrusion.

**Semantics.** In local space, `height` is either **arc length** or **chord span** between end planes (pick one convention per implementation and document it). **Top** / **bottom** are the two ends. **Width** and **depth** are cross-section extents orthogonal to the local spine tangent; the **bow** lies primarily in a chosen plane so parents and rigs can align the bend predictably.

## Parameters

**API note:** **Suggestive** until implementation locks types. The spine will likely become a spline or polynomial offset from a reference line; width falloff may match polynomial taper ideas used on [Drumstick](../drumstick/README.md).

- `height`: extent along the segment (chord or arc length—specify in code).
- `width_proximal`, `width_distal`: cross-section width at the two ends (or a single `end_width` if symmetric).
- `width_mid` (or `max_width`): width at the broadest part of the bow.
- `depth` (or per-end depths if non-circular).
- `bow_amplitude`: lateral (in-bend-plane) displacement of the spine at midsegment, or a normalized scalar driving curvature.
- `bow_asymmetry`: optional skew so widen-and-bend favors one end.

**Optional:** higher-order width curves along the arc; twist of the cross-section frame along the spine.

## Crozon feature uses

- [Lower limb](../../lower-limb/bow/README.md)
