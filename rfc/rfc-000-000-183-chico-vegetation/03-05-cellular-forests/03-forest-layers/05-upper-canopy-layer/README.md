# 3.5.3.5: Upper Canopy Layer

This page is subsection **3.5.3.5** of [RFC-183: Chico Vegetation](../../../README.md)

The upper canopy layer selects from [Well-known Upper Canopy Groves](../../../03-04-cellular-groves/03-04-07-well-known-upper-canopy-groves/README.md). It defines the dominant tree layer of the forest cell: skyline trees, large palms, orchard crowns, savanna trees, upper conifers, and massive canopy systems.

The upper canopy layer has no sublayers. It is a single distribution:

```rust
pub type UpperCanopyLayer = [
    (None, 2.0),
    (UpperCanopyGroveA, 1.0),
    (UpperCanopyGroveB, 0.25),
];
```

Selection uses [Bucket Throw](../../../03-04-cellular-groves/03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md). If the result is `None`, the forest cell remains open above the lower layers. Otherwise, the selected upper canopy grove provides the largest vertical structure in the cell.

This layer should generally be the highest ecological commitment in a forest layering. Once an upper canopy type is chosen, lower canopy, understory, tufts, and ground cover should be authored to support it rather than contradict it.
# 3.5.2.4: Upper Canopy Layer

This page is subsection **3.5.2.4** of [RFC-183: Chico Vegetation](../../../README.md)


No sublayers. Selects from upper canopy groves.