# Egg

**Figure (TBD):** add a reference image at `./assets/egg.jpeg` when you have a sketch.

A closed **ovoid**: **prolate** body with a **blunt wide** end and a **narrower** opposite pole along the long axis—reads as a **natural egg**, **teardrop** (if tip sharper), or **stylized pod** without the symmetry of a generic [Spheroid](../spheroid/README.md) on all three axes. Implement as a **surface of revolution**, **stretched ellipsoid** with asymmetric poles, or **low-poly** faceted proxy—document which.

**Semantics.** **`long_axis`** (often labeled **top** → **bottom**) runs from the **narrow** pole toward the **wide** pole (or the reverse—fix per species). **Equator** is the plane of maximum **girth** orthogonal to that axis. **Top** / **bottom** label attachment: e.g. narrow end toward **jaw** for a head egg, wide end at **cranium**.

## Parameters

**API note:** **Suggestive.** Profile curves may become splines; equator may be elliptical.

- `length`: span along the long axis (full extent).
- `equator_radius` (or `max_girth` / paired `equator_width`, `equator_depth`): maximum cross-section radius or half-extents at the **equator**.
- `pole_narrow_ratio`: ratio of **narrow-pole** effective radius to `equator_radius` (typical range `0.2`–`0.5` for egg-like silhouettes).
- `pole_wide_ratio`: optional ratio at the **blunt** pole if not implicitly ~1.0 vs equator.
- `center_offset`: translation if mesh origin is not the volume centroid.

**Optional:** `tip_sharpness` to push from **egg** toward **teardrop**; noise on hull.

## Crozon feature uses

- [Head shape](../../head-shape/egg/README.md)
- [Ear](../../ear/egg/README.md)
- [Eye](../../eye/egg/README.md)
- [Hand and foot](../../hand-foot/egg/README.md)
