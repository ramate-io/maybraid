# Half-spheroid

**Figure (TBD):** add a reference image at `./assets/half-spheroid.jpeg` when you have a sketch.

A **sphere or ellipsoid clipped by a plane** (or a **hemisphere** with optional squash)—a dome, pad, or cap without the hidden lower volume. Use where a full [Spheroid](../spheroid/README.md) would waste interior mesh or poke through a parent surface (palms, soles, brows, hoof pads).

**Semantics.** Define a **clip plane** relative to the parent frame (e.g. normal toward **outward** surface normal of the hand). **Radius** / semi-axes match [Spheroid](../spheroid/README.md); **`clip`** or **`hemisphere_ratio`** (suggestive) selects how much of the ball remains above the plane (`0.5` ≈ true half).

## Parameters

**API note:** **Suggestive.** Implement as cap mesh, boolean clip, or analytic dome; parameters stay abstract.

- `radius` (or `rx`, `ry`, `rz`): same convention as [Spheroid](../spheroid/README.md).
- `clip_height` or `cap_fraction`: distance from center to plane, or normalized “how much ball to keep.”
- `center_offset`: offset of the sphere center from the attachment frame (e.g. bulge palm outward).

**Optional:** flat **rim** width for stylized pads; facet count.

## Crozon feature uses

- [Hand and foot](../../hand-foot/half-spheroid/README.md)
