# 3.4.1.2: Density

This page is subsection **3.4.1.2** of [RFC-183: Chico Vegetation](../../../README.md)


Controls planting frequency.

* Grove defines `[min, max]`
* Forest samples via FBM

```rust
let density = fbm(world_pos * density_freq).remap(min_density, max_density);
```

Used as the activation threshold for cells.

---

