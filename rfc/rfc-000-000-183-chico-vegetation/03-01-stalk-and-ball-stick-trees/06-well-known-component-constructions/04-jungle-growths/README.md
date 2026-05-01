# 3.1.6.4: Jungle Growths

This page is subsection **3.1.6.4** of [RFC-183: Chico Vegetation](../../../README.md)


Jungle growths are secondary foliage allocations placed at selected ball points.

At a selected canopy node:

```rust
spawn_canopy_ball(node);

spawn_noisy_ball(
    position = node.position,
    radius = node.radius * jungle_growth_scale,
    material = darker_leaf_material,
);

spawn_tuft(
    position = node.position,
    direction = outward_or_upward_bias(node),
);
```

The larger, darker ball gives depth and density. The tuft adds protruding detail and a wet, overgrown silhouette.

This construction is useful for tropical trees, banyans, branch epiphytes, and dense understory vegetation.

---

