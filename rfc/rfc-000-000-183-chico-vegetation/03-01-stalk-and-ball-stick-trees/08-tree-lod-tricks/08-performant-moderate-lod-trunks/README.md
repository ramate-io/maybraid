# 3.1.8.8: Performant Moderate-LOD Trunks

This page is subsection **3.1.8.8** of [RFC-183: Chico Vegetation](../../../README.md)


Use lower sample-rate [Noisy Cylinder](../../01-stick-and-stalk-components/01-noisy-cylinder/README.md#3111-noisy-cylinder).

```rust
NoisyCylinder {
    noise_frequency: lower,
    mesh_resolution: reduced,
}
```

This preserves trunk character while reducing vertex count.

---

