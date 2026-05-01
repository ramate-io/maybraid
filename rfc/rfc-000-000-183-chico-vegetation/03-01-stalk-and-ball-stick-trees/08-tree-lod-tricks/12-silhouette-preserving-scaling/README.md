# 3.1.8.12: Silhouette-Preserving Scaling

This page is subsection **3.1.8.12** of [RFC-183: Chico Vegetation](../../../README.md)


At lower LODs, slightly exaggerate large-scale proportions to preserve readability:

* widen canopy by a small factor
* slightly shorten trunk
* reduce taper

```rust
let canopy_scale = 1.05..1.15;
let trunk_scale = 0.9..0.95;
```

This compensates for the loss of fine structure and prevents trees from appearing thin or brittle at distance.

---

