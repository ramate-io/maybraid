# 3.1.10.1: Base Leaf Palette

This page is subsection **3.1.10.1** of [RFC-183: Chico Vegetation](../../../README.md)


Each species provides a base palette for foliage color.

```rust
pub struct LeafPalette {
    pub colors: Vec<Vec3>,
    pub regional_scale: f32,
    pub detail_scale: f32,
    pub value_strength: f32,
}
```

World-space noise selects and modulates the base color:

```wgsl
let regional = fbm(world_position.xyz * leaf.regional_scale, leaf.seed);
let detail = fbm(world_position.xyz * leaf.detail_scale, leaf.seed + 101u);

let base = palette_sample(regional);
let value = mix(1.0 - leaf.value_strength, 1.0 + leaf.value_strength, detail);
```

