# 3.4.1.5: Elevation Constraints

This page is subsection **3.4.1.5** of [RFC-183: Chico Vegetation](../../../README.md)


Defines allowable elevation ranges per variant.

Elevation constraints live on the **variants** in the grove's unified cell type, not on the grove as a whole. Each bucketed variant carries its own acceptable elevation range.

```rust
pub enum GroveCell {
    LowlandTree(Bucket {
        weight: 1.0,
        placement_constraints: PlacementConstraints {
            elevation: 0.0..0.45,
            steepness: 0.0..0.30,
        },
        item: TreeVariant,
    }),
}
```

The grove still controls density, cell sizing, offsets, and distribution. It does not define a single shared elevation range for every variant.

Selection uses a first-fit placement strategy:

* sample the bucket distribution to choose a starting variant
* evaluate that variant's elevation constraint at the candidate point
* if it fails, move to the adjacent bucket in distribution order
* continue until a valid variant is found or the bucket list is exhausted

These are evaluated exactly as in terrain detail placement in
[RFC-170](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-170-terrain-detail#313-position-selection-and-validation).

This preserves spatially coherent distribution while allowing nearby variants to absorb placements that are unsuitable for the initially sampled variant.

---

