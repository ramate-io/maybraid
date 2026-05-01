# 3.5.3.1: Ground Cover Layer

This page is subsection **3.5.3.1** of [RFC-183: Chico Vegetation](../../../README.md)

The ground cover layer selects from [Well-known Ground Cover Groves](../../../03-04-cellular-groves/03-04-03-well-known-ground-cover-groves/README.md). It is the lowest vegetation layer and is responsible for terrain-hugging materials: moss, flecks, low mats, mounds, exposed floor texture, and similar surface vegetation.

Unlike the other forest layers, ground cover has two sublayers:

* **Flip**: the primary ground cover pass.
* **Flop**: an optional secondary pass that can overlap the first.

This gives the forest a simple way to express mixed ground surfaces without requiring a special combined grove for every possible pairing. For example, a forest cell may select one low mat in `flip` and one mound or flecking pass in `flop`.

```rust
pub struct GroundCoverLayer {
    flip: [
        (None, 1.0),
        (GroundCoverGroveA, 2.0),
        (GroundCoverGroveB, 1.0),
    ],
    flop: [
        (None, 4.0),
        (GroundCoverGroveC, 1.0),
    ],
}
```

Both sublayers use [Bucket Throw](../../../03-04-cellular-groves/03-04-02-selection-and-placement/03-04-02-01-bucket-throw/README.md) independently. `None` should usually be more common in `flop` than in `flip`, because the secondary pass is for accents and overlap rather than mandatory coverage.

Ground cover should not be used for upright grasses or bushes. Those belong in the [Tufts Layer](../02-tufts-layer/README.md) or [Understory Layer](../03-understory-layer/README.md).
# 3.5.2.1: Ground Cover Layer

This page is subsection **3.5.2.1** of [RFC-183: Chico Vegetation](../../../README.md)


The ground cover layer, unlike other layers is composed of two sublayers to enable a simple model of overlapping ground cover. The sublayers are referred to as Flip and Flop. 

Selects from [Ground Cover Groves](../../../03-04-cellular-groves/03-04-03-well-known-ground-cover-groves/README.md).
