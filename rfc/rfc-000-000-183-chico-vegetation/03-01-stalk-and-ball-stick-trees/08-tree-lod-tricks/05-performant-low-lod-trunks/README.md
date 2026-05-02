# 3.1.8.5: Performant Low-LOD Trunks

This page is subsection **3.1.8.5** of [RFC-183: Chico Vegetation](../../../README.md)


Use a hexagonal prism.

```rust
spawn_mesh(hex_prism(height, radius));
```

This gives:

* cylindrical impression
* low polygon count
* good normal interpolation

---

