# 3.4.8: Grove LOD Tricks

This page is subsection **3.4.8** of [RFC-183: Chico Vegetation](../../README.md)

Grove LOD tricks reduce the cost of a grove while preserving the broad impression of the planting pattern. At grove scale, the most important distant cue is not the exact number of plants, but the horizontal mass, skyline rhythm, and color texture that imply a coherent grove.

## Very Low LOD Tree Line

At very low LOD, a grove can allocate fewer tree instances while increasing their horizontal scale. This preserves the visual impression of a distant tree line without paying for every individual tree.

```rust
let lod_density = base_density * 0.20;
let lod_horizontal_scale = base_horizontal_scale * 1.75;
let lod_vertical_scale = base_vertical_scale * 0.90;
```

The vertical scale should usually change less than the horizontal scale. A distant tree line needs broad canopy mass, but overly tall replacements can make the forest silhouette drift.

## Preserve Grove Footprint

LOD should preserve the grove's footprint and distribution character:

* Keep the same grove cell grid where possible.
* Keep bucket selection deterministic for the grove cell.
* Drop instances by density thinning, not by changing the grove's identity.
* Expand surviving instances mostly in horizontal canopy spread.
* Preserve average color and canopy/stick palette balance.

## Layer-Specific Use

This trick works best for lower canopy and upper canopy groves. It is less appropriate for ground cover or tufts, where individual instances are already small, and the better strategy is usually to drop the layer at the forest level.

For understory, use it carefully: larger bushes can imply distant mass, but oversized understory can look like the wrong vegetation layer.

## Constraints

LOD scaling should happen after placement constraints have already selected valid variants. It should not allow a grove to bypass elevation, steepness, or first-fit placement behavior.
