# Contributing

Guide for contributing to the Durham terrain models layer.

## Safe cellular generation

Terrain and jersey stamps are generated per cell and composed across neighbors.
Seams appear when two adjacent cells disagree about height at a shared face.
Keep cells **safe**: a modulation owned by cell A must not change elevation on
or outside A's boundary in a way that cell B (which may not pull A) cannot
reproduce.

Practical rules:

1. **Bound softmask / falloff to the owning cell.** Softmask inside the tile is
   fine; strength should reach identity by the cell face (or a known interior
   apron). Do not rely on neighbors discovering spill.
2. **Discover with closed coverage + a small halo when influence can reach.**
   Half-open tiling plus `intersects` drops face-adjacent tiles. Prefer inclusive
   face overlap; add a Moore halo only when reach can exceed one face.
3. **Compose in a deterministic order.** Sort region results by `Id` before
   applying non-commutative elevation ops so neighboring Terrain cells see the
   same sequence.
4. **Mesh extract may still notch on sharp ridges.** Presentation cells use a
   multi-voxel apron on the cascade chunk (horizontal plus a slope-scaled
   vertical band), so neighbors share a sample strip. That hides extract gaps;
   it does not fix SDF disagreement.

When in doubt: two Terrain cells that share a face must evaluate the same world
`(x, z)` height if both have finished generation with the same universal deps.
