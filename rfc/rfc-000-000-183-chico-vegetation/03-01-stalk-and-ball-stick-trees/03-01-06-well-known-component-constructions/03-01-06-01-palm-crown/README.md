# 3.1.6.1: Palm Crown

This page is subsection **3.1.6.1** of [RFC-183: Chico Vegetation](../../../README.md)


A palm crown is built from several radially projecting frond rings placed in quick vertical succession.

Each ring places frond anchors around a central crown point:

```rust
for ring in 0..ring_count {
    let h = ring as f32 * ring_spacing;
    let vertical_bias = base_bias + ring as f32 * bias_step;

    for i in 0..fronds_per_ring {
        let theta = TAU * i as f32 / fronds_per_ring as f32;
        let radial = Vec3::new(theta.cos(), 0.0, theta.sin());

        spawn_frond(
            anchor = crown + Vec3::Y * h,
            direction = normalize(radial + Vec3::Y * vertical_bias),
        );
    }
}
```

Higher rings should start with greater upward bias. Lower rings may droop or project closer to horizontal. This produces the layered crown silhouette common to palms.

---

