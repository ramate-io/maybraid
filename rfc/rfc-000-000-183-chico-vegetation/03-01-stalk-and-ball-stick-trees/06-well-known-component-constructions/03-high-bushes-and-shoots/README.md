# 3.1.6.3: High-bushes and Shoots

This page is subsection **3.1.6.3** of [RFC-183: Chico Vegetation](../../../README.md)


High-bushes and shoots are trunkless radial constructions.

Use a ground or near-ground anchor and emit a single ring of upward-biased radial projections:

```rust
for i in 0..shoot_count {
    let theta = TAU * i as f32 / shoot_count as f32;
    let radial = Vec3::new(theta.cos(), 0.0, theta.sin());

    let dir = normalize(radial * radial_strength + Vec3::Y * vertical_bias);

    grow_chain(anchor, dir);
}
```

This construction is useful for bushes, young trees, tall grass-like woody growth, and vine-like shrubs. Leaf allocation is usually dense near terminal nodes.

---

