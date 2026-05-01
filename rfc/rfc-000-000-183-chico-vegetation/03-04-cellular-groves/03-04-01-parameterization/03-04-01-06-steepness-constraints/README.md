# 3.4.1.6: Steepness Constraints

This page is subsection **3.4.1.6** of [RFC-183: Chico Vegetation](../../../README.md)


Similar to elevation, but based on terrain slope.

```rust
let steepness = laplacian(terrain, position);
```

Steepness constraints also live on **variants** in the grove's unified cell type. A grove with mixed vegetation can therefore put different variants on different terrain:

* flatter lowland or bed variants can require low steepness
* scrub, tuft, or rocky variants can allow steeper placement
* tree-like variants can keep stricter slope limits than surrounding filler vegetation

```rust
placement_constraints: PlacementConstraints {
    elevation: 0.0..0.75,
    steepness: 0.0..0.30,
}
```

As with elevation, placement uses first-fit fallback through the bucket distribution. If the sampled variant fails the steepness check, placement tries the adjacent bucketed variant before rejecting the point.

As with elevation, this mirrors the validation step in RFC-170 terrain detail placement.

---

