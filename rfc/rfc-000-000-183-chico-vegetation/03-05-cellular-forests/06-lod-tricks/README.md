# 3.5.6: Forest LOD Tricks

This page is subsection **3.5.6** of [RFC-183: Chico Vegetation](../../README.md)

Forest LOD tricks reduce the number of active layers and grove grids while preserving the large-scale impression of a forest. At forest scale, the goal is to keep the base colors and tree-line shape that identify a biome, not to preserve every layer equally.

## Selective Layer Dropout

At low LOD, forest cells may drop layers selectively after the forest layering has been selected. Particular forest cells may use different dropout policies according to their layering: a meadow, jungle, orchard, desert, and taiga should not all simplify in the same way.

Common dropout order:

1. **Tufts** drop first.
2. **Understory** drops next.
3. **Lower canopy** thins or drops depending on forest type.
4. **Ground cover** remains as low-lying color or texture impression.
5. **Upper canopy** remains longest because it preserves skyline and forest mass.

This keeps distant forests readable: ground cover supplies broad terrain color, while canopy layers supply height and silhouette. Base color and tree-line shape are usually the highest-value signals.

```rust
pub struct ForestLodMask {
    ground_cover: LodLayerMode,
    tufts: LodLayerMode,
    understory: LodLayerMode,
    lower_canopy: LodLayerMode,
    upper_canopy: LodLayerMode,
}

pub enum LodLayerMode {
    Full,
    Thinned(f32),
    Impression,
    Dropped,
}
```

## Low-Lying Impressions

Ground cover should often remain as an impression even when individual ground-cover groves are too expensive. The renderer may replace dense ground-cover cells with simplified color, flecking, or low relief.

```rust
ground_cover = Impression;
tufts = Dropped;
understory = Dropped;
lower_canopy = Thinned(0.35);
upper_canopy = Full;
```

This is useful for distant hillsides where a surface still needs to read as grass, scrub, moss, or dry ground, but individual tufts and bushes would be visually noisy.

## Canopy Preservation

Upper canopy and sometimes lower canopy should survive farther than tufts and understory. They define the tree line and vertical structure.

When canopy cost is too high, prefer grove-level LOD tricks such as fewer trees with greater horizontal scale. See [Grove LOD Tricks](../../03-04-cellular-groves/08-grove-lod-tricks/README.md).

## Layer-Specific Rules

Layer dropout should be authored conservatively:

* Open grassland may keep ground cover and drop all canopy.
* Dense forest may keep upper canopy and drop tufts/understory first.
* Jungle may keep lower canopy longer than ordinary temperate forest.
* Orchard or vineyard may keep cultivated upper-canopy rows longer than wild undergrowth.
* Desert may drop almost everything and keep only ground-cover impressions.

LOD should not change the selected forest layering. It only changes which selected layers are evaluated or simplified at a given distance. The dropout mask can vary by forest cell or forest layering, but should preserve the cell's broad color impression and canopy outline whenever possible.
