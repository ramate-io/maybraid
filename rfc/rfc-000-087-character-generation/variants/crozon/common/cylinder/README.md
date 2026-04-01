# Cylinder

**Figure (TBD):** add a reference image at `./assets/cylinder.jpeg` when you have a sketch.

A straight **right circular or elliptical cylinder** (or box-prism stand-in) along one axis—the minimal volumetric primitive for an elongated segment.

**Semantics.** In local space, `height` runs along the **long axis**. `width` and `depth` are cross-section extents orthogonal to `height` (circle: `width == depth`); keep half-extents vs full width consistent with the generator. **Top** / **bottom** label the two caps.

**Lateral wall bow (optional).** By default, side walls are **parallel** along `height` (uniform cross-section). You can instead **bow the lateral walls** in a plane that contains the long axis and one cross-section axis (the **bow plane**): walls **pinch toward each other** midsegment (concave **waist**) or **flare outward** (convex **barrel**). The orthogonal cross-section axis may stay fixed, scale with the same law, or follow a separate rule—document which in the generator. This is **not** the same as [Bow](../bow/README.md): here the **spine stays straight**; only the **cross-section width profile** (or one pair of faces on a prism) varies along `height`.

## Parameters

**API note:** **Suggestive.** Parallel cylinder may be a prism, capped tube, or extrusion. Wall bowing may later be expressed as splines or polynomials over normalized height (similar intent to drumstick bulge taper).

- `height`: total extent along the long axis.
- `width`: cross-section **width** in the bow plane at the **reference** sample (often **top**, **bottom**, or **mid**—pick one and keep it consistent).
- `depth`: cross-section depth (orthogonal to `height` and to the bow plane’s width direction); for a circular section, match `width` or ignore one parameter per convention.
- `lateral_wall_bow`: signed control for **non-parallel lateral walls** along `height`. `0` = straight prism/cylinder as above. **Negative** = walls pinch together toward mid-height (narrower waist). **Positive** = walls bow apart mid-height (barrel / flare). Magnitude is suggestive until mapped to a real curve (e.g. `−1..1` driving a quadratic or cosine pinch).
- `lateral_wall_bow_curve`: optional placeholder for **how** bowing is distributed along the axis (e.g. bias pinch toward **top** vs **bottom** vs centered); expect replacement by explicit coefficients or a spline.

**Optional:** slight noise or facet count for render style; does not change the abstract parameters above.

## Crozon feature uses

- [Lower limb](../../lower-limb/cylinder/README.md)
- [Neck](../../neck/cylinder/README.md) (pinched / straight / flared column)
