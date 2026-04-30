# Well-known Forests

This file is part of [RFC-183: Chico Vegetation](../../../README.md).

**Construction type:** cellular forest (see section 3.5.3 in the main RFC).


```rust
pub enum ForestCell {
    Riparian,
    Chaparral,
    Alpine,
    TemperateConiferous,
    Orchard,
    Coniferous,
    Jungle,
    TropicalJungle,

}
```
