# Square Pyramid

**Figure (TBD):** add a reference image at `./assets/square-pyramid.jpeg` when you have a sketch.

A **square pyramid**: a square **base** and four triangular faces meeting at an **apex**. For limbs, think of a tapering shank—broad at one end, sharp or blunt at the other—not a regular tetrahedron (which has four triangular faces only).

**Semantics.** In local space, **`height`** is the distance from the **base plane** to the **apex** along the pyramid axis. For lower-limb use, orient the **apex toward the distal tip** of the segment (and the base toward the proximal joint), unless a species overrides that. **Base** half-width / half-depth (or a single `base_size` if square) lie in the base plane; **top** / **bottom** labels follow the limb: **proximal** ↔ base side, **distal** ↔ apex side when using the recommended orientation.

## Parameters

**API note:** **Suggestive.** Apex truncation (frustum), rounded apex, and non-square bases may appear later as extra fields or alternate variants.

- `height`: apex-to-base distance along the pyramid axis.
- `base_width`, `base_depth`: edge lengths (or full spans—match generator convention) of the square or rectangular base.
- `apex_blunt`: optional placeholder for a **truncated** apex or **bevel** at the tip (`0` = sharp corner, `> 0` = flat or rounded cap).
- `twist`: optional rotation of the base relative to a parent frame about the axis (degrees or radians—document in code).

**Optional:** facet shading via vertex normals; geodesic-style subdivision driven by a separate LOD parameter.

## As a Lower Limb

Orient the **apex** toward the **distal** end of the limb, so the limb reads as **tapering** toward the foot or claw. Elongated pyramids (large `height` relative to base) suit **lanky** legs; **squat** ratios can read as stylized or juvenile. The hard planes often sit more comfortably in **geodesic** or faceted art direction than in smooth, realistic shading.
