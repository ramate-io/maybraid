# 3.1.10.2: Flecks

This page is subsection **3.1.10.2** of [RFC-183: Chico Vegetation](../../../README.md)


A fleck is an additional color contribution applied over the base leaf color. Flecks are used for snow, buds, flowers, disease, dryness, or other localized seasonal effects.

```rust
pub struct LeafFleck {
    pub color: Vec3,
    pub strength: f32,

    pub season_center: f32,
    pub season_width: f32,
    pub season_cutoff: f32,

    pub longitude_divisor: f32,
    pub altitude_divisor: f32,

    pub season_weight: f32,
    pub longitude_weight: f32,
    pub altitude_weight: f32,

    pub noise_scale: f32,
    pub noise_cutoff: f32,
}
```

Each fleck computes a likelihood or strength from:

* season
* longitude
* altitude
* local world-space noise

The hard cutoff ensures the fleck can fully disappear rather than merely fade.

