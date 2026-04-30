# 3.1.10.5: Usage

This page is subsection **3.1.10.5** of [RFC-183: Chico Vegetation](../../../README.md)


**Snow**

Use white flecks with strong season, latitude or longitude, and altitude weighting.

```rust
LeafFleck {
    color: Vec3::splat(0.95),
    strength: 0.8,
    season_center: winter,
    season_width: winter_width,
    season_cutoff: 0.2,
    longitude_weight: 0.4,
    altitude_weight: 0.6,
    season_weight: 1.0,
    noise_cutoff: 0.45,
}
```

Trees in the same region should generally share similar snow fleck parameters regardless of species, unless understory shielding or grove-specific effects apply.

**Spring buds**

Use bright green, yellow, pink, or white flecks early in the season. Bias primarily by season with slight longitude variation.

```rust
LeafFleck {
    color: bud_color,
    strength: 0.5,
    season_center: early_spring,
    season_width: short,
    season_weight: 1.0,
    longitude_weight: 0.2,
    altitude_weight: 0.1,
    noise_cutoff: 0.55,
}
```

**Overlapping flecks**

Multiple flecks may overlap. Their `strength` controls how aggressively each fleck blends over the current color. This lets snow, buds, flowers, and leaf-color variation coexist without requiring separate materials.

