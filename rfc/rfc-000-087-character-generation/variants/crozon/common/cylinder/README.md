# Cylinder

**Figure (TBD):** add a reference image at `./assets/cylinder.jpeg` when you have a sketch.

A straight **right circular or elliptical cylinder** (or box-prism stand-in) along one axis—the minimal volumetric primitive for an elongated segment.

**Semantics.** In local space, `height` runs along the **long axis**. `width` and `depth` are cross-section extents orthogonal to `height` (circle: `width == depth`); keep half-extents vs full width consistent with the generator. **Top** / **bottom** label the two caps.

## Parameters

**API note:** **Suggestive** only in the sense that “cylinder” might be implemented as a capped tube, prism, or extrusion; names stay stable.

- `height`: total extent along the long axis.
- `width`: cross-section width (orthogonal to `height`).
- `depth`: cross-section depth (orthogonal to `height`); for a circular section, match `width` or ignore one parameter per convention.

**Optional:** slight noise or facet count for render style; does not change the abstract parameters above.

## Crozon feature uses

- [Lower limb](../../lower-limb/cylinder/README.md)
