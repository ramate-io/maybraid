# 3.1.7.17: Simpleman's Tuft

This page is subsection **3.1.7.17** of [RFC-183: Chico Vegetation](../../../README.md)


Simpleman's Tuft is the most basic ground vegetation construction. It consists of a single [Tuft](../../03-01-02-ball-components/03-01-02-06-tufts/README.md#3126-tufts) placed directly on terrain.

**Shape**

* Small jagged vegetation clump
* No stalk or branch graph
* SDF-backed tuft geometry
* Suitable for ground cover and small plants

**Construction**

```rust
spawn_tuft(
    position = terrain_position,
    direction = terrain_normal,
    scale = tuft_scale,
);
```

Use deterministic scale and rotation variation:

```rust
let scale = mix(min_scale, max_scale, noise(seed, SCALE_SALT));
let yaw = TAU * noise(seed, ROTATION_SALT);
```

**Materials**

* Leaf shader: grass, scrub, jungle undergrowth, dry brush, or flowering ground cover

**Variants**

* Increase scale for small bushes.
* Use dense placement for ground cover.
* Combine with [Simpleman's Hedge](../03-01-07-16-simpleman-s-hedge/README.md#31716-simplemans-hedge) for layered shrubbery.

