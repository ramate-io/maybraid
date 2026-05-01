# 3.5.3.2: Tufts Layer

This page is subsection **3.5.3.2** of [RFC-183: Chico Vegetation](../../../README.md)

The tufts layer selects from [Well-known Tufts Groves](../../../03-04-cellular-groves/03-04-04-well-known-tufts-groves/README.md). It covers small upright vegetation above the ground surface: grasses, tuft clusters, reeds, low scrubby tuft forms, and other repeated vertical texture.

The tufts layer has no sublayers. It is a single distribution:

```rust
pub type TuftsLayer = [
    (None, 2.0),
    (TuftsGroveA, 1.0),
    (TuftsGroveB, 1.0),
];
```

Selection uses [Bucket Throw](../../../03-04-cellular-groves/03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md). If the result is `None`, the forest cell has no tuft pass. Otherwise, the selected tuft grove is instantiated with its own cell size, density, offset, noise, and placement constraints.

Tufts should be visually subordinate to understory. They can overlap ground cover freely, but they should not be used for large bushes, young trees, or lower canopy fill.
# 3.5.2.2: Tufts Layer

This page is subsection **3.5.2.2** of [RFC-183: Chico Vegetation](../../../README.md)


No sub layers. Selects from tufts. 