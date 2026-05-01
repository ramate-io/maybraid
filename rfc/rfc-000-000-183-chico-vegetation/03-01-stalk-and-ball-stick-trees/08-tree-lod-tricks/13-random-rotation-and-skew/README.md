# 3.1.8.13: Random Rotation and Skew

This page is subsection **3.1.8.13** of [RFC-183: Chico Vegetation](../../../README.md)


Introduce small deterministic variation in orientation and scale to break repetition across instances.

**Rotation**

Rotate around the vertical axis:

```rust
let yaw = TAU * noise(seed, ROT_SALT);
transform.rotate_y(yaw);
```

This prevents aligned silhouettes across large forests.

**Non-uniform scale (skew-like effect)**

Apply slight variation in horizontal axes:

```rust
let sx = 0.9 + 0.2 * noise(seed, SCALE_X);
let sz = 0.9 + 0.2 * noise(seed, SCALE_Z);

transform.scale *= Vec3::new(sx, 1.0, sz);
```

This produces:

* slight elongation or compression
* variation in canopy footprint
* reduced tiling artifacts

**Optional lean (very subtle)**

```rust
let lean = 0.05 * noise(seed, LEAN_SALT);
transform.rotate_axis(Vec3::Z, lean);
```

Use sparingly; excessive lean breaks vertical readability.

These small variations are critical for avoiding visual repetition when using low-LOD primitives.

---

