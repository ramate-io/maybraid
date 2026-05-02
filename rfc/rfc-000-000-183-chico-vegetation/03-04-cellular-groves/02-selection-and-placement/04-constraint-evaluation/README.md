# 3.4.2.4: Constraint Evaluation

This page is subsection **3.4.2.4** of [RFC-183: Chico Vegetation](../../../README.md)


Evaluate terrain at the selected point.

```rust
let elevation = terrain_height(p);
let steepness = laplacian(terrain, p);
```

Reject placements that violate constraints.

```rust
fn valid_at(p: Vec3, variant: &GroveVariant) -> bool {
    let c = variant.placement_constraints;

    within(elevation, c.elevation)
        && within(steepness, c.steepness)
}
```

Constraints are evaluated against the **candidate variant**, not the grove as a whole. If the starting variant fails, selection advances to the adjacent bucketed variant and tries again at the same candidate point:

```rust
for variant in distribution.first_fit_from(start_bucket) {
    if valid_at(p, variant) {
        place(variant, p);
        break;
    }
}
```

This is directly analogous to the validation phase in RFC-170 terrain detail, but the failure path stays inside the grove distribution before the point is rejected.

---

