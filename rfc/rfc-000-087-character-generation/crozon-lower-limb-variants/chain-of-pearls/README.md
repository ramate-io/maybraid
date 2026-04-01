# Chain of Pearls

**Figure (TBD):** add a reference image at `./assets/chain-of-pearls.jpeg` when you have a sketch.

A lower-limb segment built as a **sequence of spheroids** (or near-spheroids) centered on a spine—beads on a string. Useful for stylized joints, larval forms, or low-poly “segmented” legs without explicit bone meshes.

**Semantics.** In local space, the **spine** runs from **proximal** (**top**) to **distal** (**bottom**) along the limb axis. Each pearl has a **center** on that spine and a **radius** (or `width`/`depth` if you squash to ellipsoids). Adjacent pearls may **overlap** slightly for a fused look or **gap** slightly for distinct beads.

## Parameters

**API note:** **Suggestive.** Count and radii might become arrays or procedural schedules; overlap might be derived from radii and spacing automatically.

- `height`: total span along the limb axis occupied by the chain (or use pearl placement rules below and derive height).
- `pearl_count`: number of spheroids along the chain.
- `radius_base` (or `radius`): nominal sphere radius; if pearls vary, treat this as the first or median pearl.
- `radius_taper`: multiplicative or additive schedule from proximal to distal (scalar placeholder until you store per-pearl radii).
- `spacing`: center-to-center distance along the spine (if omitted, infer from radii and `overlap`).
- `overlap`: normalized or absolute overlap between neighbors (`0` = tangent spheres, `> 0` = intersection depth).
- `squash`: optional `width`/`depth` scale if pearls are ellipsoids instead of spheres.

**Optional:** jitter per pearl (radius, lateral wobble) for seeded variety; geodesic or noise on the hull.

## As a Lower Limb

Chain-of-pearls limbs read best when **silhouette rhythm** matters more than a single continuous muscle volume. Anchor **proximal** and **distal** sockets to the first and last pearl centers (or hull extremes) so the rig chain stays predictable.
