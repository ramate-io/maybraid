# Chain of pearls

**Figure (TBD):** add a reference image at `./assets/chain-of-pearls.jpeg` when you have a sketch.

A **sequence of spheroids** (or near-spheroids) centered on a spine—beads on a string. Useful for stylized segmentation, larval or tentacle-like forms, and low-poly “joint stacks” without explicit bone geometry.

**Semantics.** In local space, the **spine** runs from **top** to **bottom** along the long axis. Each pearl has a **center** on that spine and a **radius** (or `width`/`depth` if squashed to ellipsoids). Neighbors may **overlap** for a fused look or **gap** for distinct beads.

## Parameters

**API note:** **Suggestive.** Count and radii might become arrays or procedural schedules; overlap might be derived from radii and spacing automatically.

- `height`: total span along the spine occupied by the chain (or derive from placement rules).
- `pearl_count`: number of spheroids.
- `radius_base` (or `radius`): nominal radius; if pearls vary, treat as first or median.
- `radius_taper`: schedule from **top** to **bottom** (scalar placeholder until per-pearl radii exist).
- `spacing`: center-to-center distance (if omitted, infer from radii and `overlap`).
- `overlap`: neighbor overlap (`0` = tangent, `> 0` = intersection depth).
- `squash`: optional `width`/`depth` scale for ellipsoids.

**Optional:** jitter per pearl (radius, lateral wobble) for seeded variety; geodesic or noise on the hull.

## Crozon feature uses

- [Lower limb](../../lower-limb/chain-of-pearls/README.md)
- [Ear](../../ear/chain-of-pearls/README.md)
- [Mouth](../../mouth/chain-of-pearls/README.md)
- [Neck](../../neck/chain-of-pearls/README.md)
- [Tail](../../tail/chain-of-pearls/README.md)
