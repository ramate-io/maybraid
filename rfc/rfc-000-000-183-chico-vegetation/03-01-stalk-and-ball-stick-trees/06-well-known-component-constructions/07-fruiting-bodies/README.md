# 3.1.6.7: Fruiting Bodies

This page is subsection **3.1.6.7** of [RFC-183: Chico Vegetation](../../../README.md)


Fruiting bodies are optional canopy details: small, brightly colored ellipsoidal volumes placed on or near the radius of canopy components. They are useful for fruit trees, jungle growths, magical trees, and seasonal variation.

A fruiting body should usually be attached to a selected canopy node or canopy ball, not to the trunk. The placement rule samples points near the canopy surface:

```rust
let dir = sample_sphere(seed, i);
let p = canopy_center + dir * canopy_radius;
```

Then apply a small inward or outward offset, so fruit appears embedded in the foliage rather than floating:

```rust
let p = p - dir * embed_depth;
```

The fruit itself can be a scaled sphere or ellipsoid:

```rust
pub struct FruitingBodyConfig {
    pub count: usize,
    pub radius: Vec3,
    pub color: Color,
    pub embed_depth: f32,
    pub surface_bias: f32,
}
```

For SDF construction:

```rust
fn ellipsoid_sdf(p: Vec3, r: Vec3) -> f32 {
    (p / r).length() - 1.0
}
```

For mesh construction, use a low-subdivision UV sphere or icosphere and apply non-uniform scale:

```rust
spawn_ellipsoid(
    position = p,
    scale = config.radius,
    material = fruit_material,
);
```

Fruit allocation should be sparse and deterministic:

```rust
for i in 0..config.count {
    if noise(seed, i) < fruit_probability {
        continue;
    }

    let dir = sample_canopy_surface(seed, i);
    let p = canopy_center + dir * canopy_radius;
    spawn_fruit(p);
}
```

A more advanced form may include seasonality. A time or season parameter can modulate both visibility and size:

```rust
let maturity = seasonal_curve(time, fruit_phase, fruit_duration);

let scale = base_scale * maturity;
let visible = maturity > visibility_threshold;
```

This allows fruit to emerge, grow, ripen, and disappear over time without changing the underlying tree construction. Color may also vary with maturity, for example green to yellow to red.

