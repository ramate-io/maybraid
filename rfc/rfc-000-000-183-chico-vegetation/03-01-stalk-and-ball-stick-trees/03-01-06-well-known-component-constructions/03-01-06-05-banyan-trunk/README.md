# 3.1.6.5: Banyan Trunk

This page is subsection **3.1.6.5** of [RFC-183: Chico Vegetation](../../../README.md)


A banyan trunk is a thick, noisy stalk.

Use a large radius and high surface noise:

```rust
let trunk = NoisyCylinder {
    base_radius: large,
    top_radius: large * taper,
    noise_amplitude: high,
    noise_frequency: medium,
};
```

Banyan trunks should appear irregular and rooted rather than smooth. Crook cylinders may be used for secondary trunk forms, but the primary impression should come from radius, noise, and mass.

Joint-concealing balls using bark material may be allocated near major trunk or branch intersections.

---

