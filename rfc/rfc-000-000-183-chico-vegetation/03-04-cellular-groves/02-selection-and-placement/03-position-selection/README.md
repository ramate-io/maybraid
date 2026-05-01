# 3.4.2.3: Position Selection

This page is subsection **3.4.2.3** of [RFC-183: Chico Vegetation](../../../README.md)


Their exact point is determined by an offset on the grid, following
[RFC-170: Terrain Detail – Position Selection and Validation](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-170-terrain-detail#313-position-selection-and-validation).

```rust
let p = cell_origin + offset;
```

---

