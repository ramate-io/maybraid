# 3.4.2.2: Cell Activation

This page is subsection **3.4.2.2** of [RFC-183: Chico Vegetation](../../../README.md)


Cells are selected based on density and noise.

```rust
if fbm(cell_pos * density_freq) > density_threshold {
    continue;
}
```

---

