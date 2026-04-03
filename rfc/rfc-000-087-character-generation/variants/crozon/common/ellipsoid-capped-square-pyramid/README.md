# Ellipsoid-capped square pyramid

**Figure (TBD):** add a reference image at `./assets/ellipsoid-capped-square-pyramid.jpeg` when you have a sketch.

A **square pyramid** family variant with rounded/curved edges that terminates in an **ellipsoidal cap** instead of a sharp apex. Think of this as a blunted pyramid whose tip region is replaced by an ellipsoid patch.

**Semantics.** In local space, `height` runs from base plane to the cap tip direction, `base_width` / `base_depth` define the square base, and the terminal cap is controlled by an ellipsoid region (`cap_radius_*` or equivalent). If the ellipsoid cap is continuously contracted to zero, the primitive limits back toward a sharp square pyramid. In that limiting behavior, two opposing side faces can read as **Reuleaux-like** triangle profiles, depending on edge-curve settings.

## Parameters

**API note:** **Suggestive.** Build from square-pyramid control frames, then blend apex neighborhood into an ellipsoid patch with curvature continuity.

- `height`: base-to-top extent along the pyramid axis.
- `base_width`, `base_depth`: base dimensions.
- `edge_curve`: curvature/fillet strength along lateral edges.
- `cap_radius_x`, `cap_radius_y`, `cap_radius_z`: ellipsoid cap radii in local frame.
- `cap_offset`: axial position where cap blending begins.
- `apex_contract`: contracts ellipsoid cap toward a point (`1` approximates classic pyramid apex).

## Crozon feature uses

- [Torso](../../torso/ellipsoid-capped-square-pyramid/README.md) (tapered trunk with softened terminal end)
- [Head shape](../../head-shape/ellipsoid-capped-square-pyramid/README.md) (exotic wedge cranium without needle apex)
