# 3.4.1.1: Scale

This page is subsection **3.4.1.1** of [RFC-183: Chico Vegetation](../../../README.md)


Controls overall tree size.

* Grove defines `[min, max]`
* Forest samples via FBM

```rust
let scale = fbm(world_pos * scale_freq).remap(min_scale, max_scale);
```

Nearby groves will have similar scales.

---

