# 3.1.8.7: Performant Moderate-LOD Canopy

This page is subsection **3.1.8.7** of [RFC-183: Chico Vegetation](../../../README.md)


Use:

* icosahedra
* icospheres (low subdivision)

Mixing both helps preserve organic variation while keeping geometry simple.

```rust
spawn_mesh(icosphere(subdivisions = 1..2));
```

---

