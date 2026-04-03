# Hex notch

**Figure (TBD):** add a reference image at `./assets/hex-notch.jpeg` when you have a sketch.

A planar-profile primitive that reads like a **hexagon** with a **triangular notch** removed from the lower edge, then extruded to depth for 3D use. It preserves a stable geometric head mass while introducing a lower negative space that can imply tendril roots, hanging appendages, or a split jaw area.

**Semantics.** In local space, `height` spans crown to chin, `width` spans left to right, and `depth` extrudes front to back. The lower edge carries a centered triangular cutout whose apex points upward into the mass. Keep the notch centered unless asymmetry is intentional.

## Parameters

**API note:** **Suggestive.** This can be generated as a 2D polygon profile plus depth extrusion, or as direct indexed mesh topology.

- `width`: full lateral span of the outer hex silhouette.
- `height`: crown-to-lower-edge span before notch subtraction.
- `depth`: front-to-back extrusion thickness.
- `notch_width`: span of the triangular notch at the lower edge.
- `notch_height`: upward depth of the notch apex into the silhouette.
- `corner_bevel`: optional rounding/chamfer of outer corners and notch corners.

## Crozon feature uses

- [Head shape](../../head-shape/hex-notch/README.md) (mildly exotic cranial silhouette with tendril-read lower cutout)
