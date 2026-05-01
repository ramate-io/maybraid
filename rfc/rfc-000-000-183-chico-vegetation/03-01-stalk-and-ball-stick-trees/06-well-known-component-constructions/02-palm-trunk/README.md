# 3.1.6.2: Palm Trunk

This page is subsection **3.1.6.2** of [RFC-183: Chico Vegetation](../../../README.md)


A palm trunk should be built without allocating a separate stalk. Instead, use a tight ball-stick chain grown upward from a ground anchor.

The chain should have:

* strong vertical bias
* low angular variance
* consistent slight directional bias for arching palms
* tight hysteresis to preserve a smooth curve

```rust
let config = HysteresisConfig {
    bias_ray: normalize(Vec3::Y + arch_bias),
    bias_strength: high,
    angle_tolerance: low,
    child_count: 1..=1,
    length_range: short..medium,
    radius_range: trunk_radius..trunk_radius,
};
```

Invert the usual tapering rule, so the bottom of each segment is slightly narrower than the top:

```rust
segment.base_radius = r * 0.92;
segment.top_radius = r;
```

Repeated over many segments, this gives the impression of stacked palm trunk bands.

---

