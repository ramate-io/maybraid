# 3.5.3.4: Lower Canopy Layer

This page is subsection **3.5.3.4** of [RFC-183: Chico Vegetation](../../../README.md)

The lower canopy layer selects from [Well-known Lower Canopy Groves](../../../03-04-cellular-groves/06-well-known-lower-canopy-groves/README.md). It covers subcanopy trees and tall intermediate growth that sit below the dominant upper canopy.

The lower canopy layer has no sublayers. It is a single distribution:

```rust
pub type LowerCanopyLayer = [
    (None, 2.0),
    (LowerCanopyGroveA, 1.0),
    (LowerCanopyGroveB, 0.25),
];
```

Selection uses [Bucket Throw](../../../03-04-cellular-groves/02-selection-and-placement/01-bucket-throw/README.md). If a lower canopy grove is selected, it is instantiated independently of the upper canopy, but the forest layering should ensure the two make ecological and visual sense together.

Use this layer for young trees, palms, subcanopy masses, lower massive forms, and cultural or transitional tree layers. Do not use it for the tallest dominant trees in the scene; those belong in the [Upper Canopy Layer](../05-upper-canopy-layer/README.md).