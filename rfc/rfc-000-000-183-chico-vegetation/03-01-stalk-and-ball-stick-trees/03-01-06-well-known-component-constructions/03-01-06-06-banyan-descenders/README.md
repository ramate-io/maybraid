# 3.1.6.6: Banyan Descenders

This page is subsection **3.1.6.6** of [RFC-183: Chico Vegetation](../../../README.md)


Banyan descenders are downward-growing branch chains emitted from the upper canopy.

Use high radial projection segment count and a chain rule that periodically switches to a strong downward bias:

```rust
fn hysteresis_for(ctx: ChainContext) -> HysteresisConfig {
    if ctx.segment_index % descender_period == 0 {
        HysteresisConfig {
            bias_ray: -Vec3::Y,
            bias_strength: very_high,
            angle_tolerance: low,
            child_count: 1..=1,
            length_range: long..very_long,
            radius_range: thin..medium,
        }
    } else {
        ordinary_canopy_config(ctx)
    }
}
```

Descenders should often extend below the canopy height and may approach or intersect the ground. When they reach the ground, they can be thickened or treated as secondary stalks.

Use sparse foliage on descenders themselves; most foliage should remain attached to the upper canopy.

