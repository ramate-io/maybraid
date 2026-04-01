# Spheroid

**Figure (TBD):** add a reference image at `./assets/spheroid.jpeg` when you have a sketch.

A single **ellipsoid** (sphere as the special case with equal axes)—the minimal closed volumetric primitive for a lump, joint cap, or stylized mass without a distinguished long extrusion.

**Semantics.** In local space, place the shape by a **center** and three **semi-axes** (or `width` / `depth` / `height` as **full** spans along orthogonal axes—match generator convention to chain-of-pearls and other spheroid uses). **Top** / **bottom** are optional labels along the **primary** axis when embedding in a limb or torso (e.g. world-up for a belly mass); document the mapping per feature.

## Parameters

**API note:** **Suggestive.** May be implemented as icosphere, UV sphere, analytic SDF, or low-poly faceting; parameters stay abstract.

- `radius` (or `rx`, `ry`, `rz`): semi-axes or full extents—pick one schema and reuse everywhere.
- `squash`: optional single scalar if you drive two axes from a third (e.g. pancake or sausage) until explicit `rx`, `ry`, `rz` exist.
- `center_offset`: optional translation in parent space if the mesh origin is not the ellipsoid center.

**Optional:** geodesic subdivision level; noise displacement on the hull.

## Crozon feature uses

- [Torso](../../torso/spheroid/README.md)
