# Cylinder

**Figure (TBD):** add a reference image at `./assets/cylinder.jpeg` when you have a sketch.

A straight **right circular or elliptical cylinder** (or box-prism stand-in) along the limb axis—the minimal volumetric primitive for a segment.

**Semantics.** In local space, `height` runs along the limb segment (long axis). `width` and `depth` are cross-section extents orthogonal to `height` (circle as special case `width == depth`); keep half-extents vs full width consistent with the generator. **Top** is **proximal** (toward the body/trunk joint); **bottom** is distal.

## Parameters

**API note:** **Suggestive** only in the sense that “cylinder” might be implemented as a capped tube, prism, or extrusion; names stay stable.

- `height`: total extent along the limb axis.
- `width`: cross-section width (orthogonal to `height`).
- `depth`: cross-section depth (orthogonal to `height`); for a circular section, match `width` or ignore one parameter per convention.

**Optional:** slight noise or facet count for render style; does not change the abstract parameters above.

## As a Lower Limb

Cylinders are best for **thin** lower limbs. The lack of silhouette detail tends to look awkward when the camera lingers on the leg, even if surface noise is applied—use sparingly or pair with strong footwear, motion, or surrounding shapes.
