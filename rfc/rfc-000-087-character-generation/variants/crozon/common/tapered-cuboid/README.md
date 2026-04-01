# Tapered cuboid

**Figure (TBD):** add a reference image at `./assets/tapered-cuboid.jpeg` when you have a sketch.

An elongated primitive in the same **family** as [Drumstick](../drumstick/README.md): a **dominant** region along the long axis plus a **transition** to a narrower continuation—but the **bulge** is read as a **cuboid** (rectangular cross-section) with **face-aligned** silhouette, not a round cylinder swell. The **non-bulge** end is typically a **smaller box** or **tapered frustum** (linearly interpolated width/depth along the axis), which maps naturally to **chest → waist**, **rib cage → abdomen**, or blocky stylized torsos.

**Semantics.** In local space, `height` runs along the trunk axis (often **spine-aligned** in a character: **top** toward shoulders/clavicle, **bottom** toward pelvis—document per species). **Width** and **depth** span the **rectangular** cross-section orthogonal to `height`. The **bulge** occupies a **cuboid** span along `height` (constant `width`/`depth` across that span unless you add corner bevels only). Below or above the bulge, cross-sections **taper** linearly or by a placeholder curve toward `shaft_width` / `shaft_depth` at the opposite cap (torso “waist” vs “chest”).

Shared vocabulary with drumstick: axial **bulge length** vs **shaft** length can use `drop` / `bulge_to_shaft_ratio` with the same algebra; the difference is **cross-section shape** (box vs circle) and **taper** usually applied to **both** width and depth for a torso block.

## Parameters

**API note:** **Suggestive.** Taper along the shaft may become polynomials per edge; corner **bevel** / **chamfer** may become separate fields.

- `height`: total extent along the trunk axis.
- `bulge_width`, `bulge_depth`: cross-section of the **cuboid bulge** (orthogonal to `height`).
- `shaft_width`, `shaft_depth`: cross-section at the **narrow** end of the taper (typically the **bottom** cap for chest-upward orientation = waist).
- `drop`: axial length of the **cuboid bulge** from the **top** reference (or from **bottom**—document once).
- `bulge_to_shaft_ratio`: same meaning as [Drumstick](../drumstick/README.md) (bulge axial length / shaft axial length) when you prefer ratio over `drop`.
- `taper_curve`: placeholder for how `width`/`depth` interpolate between bulge and shaft along the non-bulbous span (`0` ≈ linear; expect splines later).
- `offset`: optional shift of the cuboid bulge along the axis relative to the **top** reference.
- `corner_bevel`: optional scalar for rounding box edges without abandoning the cuboid read.

**Optional:** asymmetric taper (e.g. wider back than front); noise on faces for low-poly breakup.

## Crozon feature uses

- [Torso](../../torso/tapered-cuboid/README.md)
