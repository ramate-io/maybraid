# 3.4.1.6: Steepness Constraints

This page is subsection **3.4.1.6** of [RFC-183: Chico Vegetation](../../../README.md)


Similar to elevation, but based on terrain slope.

```rust
let steepness = laplacian(terrain, position);
```

* Grove defines acceptable range
* Forest perturbs slightly

As with elevation, this mirrors the validation step in RFC-170 terrain detail placement.

---

