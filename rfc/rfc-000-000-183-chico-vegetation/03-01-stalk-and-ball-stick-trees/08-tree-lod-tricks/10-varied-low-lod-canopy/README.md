# 3.1.8.10: Varied Low-LOD Canopy

This page is subsection **3.1.8.10** of [RFC-183: Chico Vegetation](../../../README.md)


Use noise to select between primitive types:

* icosahedron
* tetrahedron

```rust
if noise(seed) < 0.5 {
    use_icosahedron();
} else {
    use_tetrahedron();
}
```

This reduces repetition across distant forests.

---

