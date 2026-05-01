# 3.5.3.3: Understory Layer

This page is subsection **3.5.3.3** of [RFC-183: Chico Vegetation](../../../README.md)

The understory layer selects from [Well-known Understory Groves](../../../03-04-cellular-groves/03-04-05-well-known-understory-groves/README.md). It covers vegetation taller and more structured than tufts but still below tree canopy: bushes, large grasses, thickets, chaparral, small scrub trees, and dense low woody growth.

The understory layer has no sublayers. It is a single distribution:

```rust
pub type UnderstoryLayer = [
    (None, 1.0),
    (UnderstoryGroveA, 1.0),
    (UnderstoryGroveB, 0.5),
];
```

Selection uses [Bucket Throw](../../../03-04-cellular-groves/03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md). A selected understory grove may overlap ground cover and tufts, but it should be authored to fit beneath any selected lower or upper canopy.

The `None` weight controls openness at walking height. Higher `None` weights create passable woodland, savanna, orchard, or sparse scrub. Lower `None` weights create thicket, jungle, or dense brush.
# 3.5.2.3: Understory Layer

This page is subsection **3.5.2.3** of [RFC-183: Chico Vegetation](../../../README.md)


No sublayers. Selects from understory.