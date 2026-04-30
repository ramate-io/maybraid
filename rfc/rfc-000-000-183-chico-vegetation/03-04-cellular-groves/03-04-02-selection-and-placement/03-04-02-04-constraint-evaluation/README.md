# 3.4.2.4: Constraint Evaluation

This page is subsection **3.4.2.4** of [RFC-183: Chico Vegetation](../../../README.md)


Evaluate terrain at the selected point.

```rust
let elevation = terrain_height(p);
let steepness = laplacian(terrain, p);
```

Reject placements that violate constraints.

```rust
if !within(elevation, elevation_range) { continue; }
if !within(steepness, steepness_range) { continue; }
```

This is directly analogous to the validation phase in RFC-170 terrain detail.

---

