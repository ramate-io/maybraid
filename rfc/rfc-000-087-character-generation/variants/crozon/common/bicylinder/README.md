# Bicylinder

**Figure (TBD):** add a reference image at `./assets/bicylinder.jpeg` when you have a sketch.

Two cylinders that read as one piece: they start in a **merged** region, **bow apart** through the middle of the segment, then **merge again** toward the opposite end. Useful for any elongated volume that should suggest **paired internal shafts** without a full topological split.

**Semantics.** In local space, `height` runs along the segment **long axis**. **Top** and **bottom** label the two ends (when embedded in a limb, map to proximal/distal as needed). The two cylinders lie mainly in a **divergence plane** (e.g. medial–lateral or front–back): separation is measured in that plane; the third axis carries the usual cross-section depth label.

## Parameters

**API note:** Names and shapes here are **suggestive** until a generator or schema fixes the real types. Blend regions and the bow curve may later become splines or explicit profile polynomials.

- `height`: total extent along the long axis.
- `merge_proximal_length`: axial length of fused/blended region at the **top** end.
- `merge_distal_length`: axial length of fused/blended region at the **bottom** end.
- `divergence_max`: peak half-separation between the two cylinder axes in the divergence plane (or a normalized `0..1` stand-in for “how far apart” at midsegment).
- `branch_radius` (or `branch_width` / `branch_depth`): characteristic cross-section of each cylinder away from the merges; keep half-extents vs full width consistent with your generator.
- `bow_bias`: optional asymmetry so the pair bows more toward one side of the divergence plane.

**Optional:** fillet/blend radius at merge zones; noise or geodesic detail on the outer hull.

## Crozon feature uses

- [Lower limb](../../lower-limb/bicylinder/README.md)
