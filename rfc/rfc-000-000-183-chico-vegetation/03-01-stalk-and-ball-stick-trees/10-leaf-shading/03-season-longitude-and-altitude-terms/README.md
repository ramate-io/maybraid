# 3.1.10.3: Season, Longitude, and Altitude Terms

This page is subsection **3.1.10.3** of [RFC-183: Chico Vegetation](../../../README.md)


Season is cyclic over a normalized year:

```rust
let season: f32; // 0..1
```

A simple cyclic season response:

```wgsl
fn cyclic_window(t: f32, center: f32, width: f32) -> f32 {
    let d = abs(fract(t - center + 0.5) - 0.5);
    return smoothstep(width, 0.0, d);
}
```

Longitude and altitude can be normalized into coarse environmental masks:

```wgsl
let lon_term = fbm(vec3<f32>(world_position.x / fleck.longitude_divisor, 0.0, 0.0), seed);
let alt_term = smoothstep(alt_min, alt_max, world_position.y);
```

The combined fleck strength is:

```wgsl
let env =
    fleck.season_weight * season_term +
    fleck.longitude_weight * lon_term +
    fleck.altitude_weight * alt_term;

let env = env / max(
    0.0001,
    fleck.season_weight + fleck.longitude_weight + fleck.altitude_weight,
);
```

Then apply local fleck noise:

```wgsl
let local = fbm(world_position.xyz * fleck.noise_scale, seed);
let mask = env * local;
```

If `mask < fleck.noise_cutoff`, the fleck is absent.

