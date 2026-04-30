# Simpleman's Hedge

This file is part of [RFC-183: Chico Vegetation](../../../README.md).

**Construction type:** tree construction (see section 3.1.7 in the main RFC).


Simpleman's Hedge is a minimal hedge construction that does not require ball-stick chains. It is built by placing [Plane Splay](../../../README.md#3125-plane-splay) components directly along the ground or along a hedge guide path.

**Shape**

* Low, dense foliage band
* No explicit stalk or branch graph
* Ground-aligned or path-aligned
* Cheap to generate and suitable for urban or garden settings

**Construction**

```rust
for p in hedge_samples(path_or_cell, spacing) {
    spawn_plane_splay(
        position = p,
        radius = hedge_radius,
        vertical_bias = Vec3::Y,
    );
}
```

Use overlapping splays to create a continuous hedge mass.

```rust
let spacing = 0.5 * hedge_radius;
let hedge_radius = 0.08 * H;
```

**Materials**

* Leaf shader: hedge green, ornamental foliage, flowering shrub variants

**Variants**

* Follow a line or polygon boundary for garden hedges.
* Scatter in cell interiors for rough shrub masses.
* Add sparse [Fruiting Bodies](../../component-construction/fruiting-bodies/README.md) for berry hedges.

---
