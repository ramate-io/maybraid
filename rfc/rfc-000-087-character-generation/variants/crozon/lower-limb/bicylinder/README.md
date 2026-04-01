# Bicylinder

**Figure (TBD):** add a reference image at `./assets/bicylinder.jpeg` when you have a sketch.

Two cylinders that read as one piece: they start in a **merged** region, **bow apart** through the middle of the segment, then **merge again** toward the distal end. The silhouette is useful when you want a single limb volume that still suggests paired bones or twin shafts without fully splitting the mesh.

**Semantics.** In local space, `height` runs along the limb segment (long axis). **Top** is **proximal** (toward the body/trunk joint); **bottom** is distal. Treat the two cylinders as lying mainly in a **divergence plane** (e.g. medial–lateral or front–back): separation is measured in that plane; the orthogonal axis stays the usual cross-section depth for labeling.

## Parameters

**API note:** Names and shapes here are **suggestive** until a generator or schema fixes the real types. Blend regions and the bow curve may later become splines or explicit profile polynomials.

- `height`: total extent along the limb axis.
- `merge_proximal_length`: axial length of the region where the two cylinders are fused or tightly blended at the proximal end.
- `merge_distal_length`: axial length of the fused/blended region at the distal end.
- `divergence_max`: peak half-separation between the two cylinder axes in the divergence plane (or a normalized `0..1` stand-in for “how far apart” at midsegment).
- `branch_radius` (or `branch_width` / `branch_depth`): characteristic cross-section of each cylinder away from the merges; keep half-extents vs full width consistent with your generator.
- `bow_bias`: optional asymmetry so the pair bows more toward one side of the divergence plane (useful for slight twist or stance read).

**Optional:** fillet/blend radius at the merge zones; noise or geodesic detail on the outer hull.

## As a Lower Limb

Bicylinders are best for exotic creatures. Exaggerating the gap between the cylinders can help the construction avoid looking too skeletal.
