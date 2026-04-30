# 3.4.1.4: Offsets

This page is subsection **3.4.1.4** of [RFC-183: Chico Vegetation](../../../README.md)


Controls intra-cell placement.

* Grove defines min and max offset ranges
* Forest selects values within that range

As discussed in [Cell Selection and Planting Constraints](../../03-04-02-selection-and-placement/README.md#342-selection-and-placement), this follows the same approach as
[RFC-170: Terrain Detail – Position Selection and Validation](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-170-terrain-detail#313-position-selection-and-validation).

```rust
let offset = noise_vec2(seed).remap(offset_min, offset_max);
let position = cell_origin + offset;
```

Offsets may exceed sub-cell bounds, but ownership and stability are always derived from the parent cell.

---

