# 3.1.9.2: World-space Variation

This page is subsection **3.1.9.2** of [RFC-183: Chico Vegetation](../../../README.md)


Use low-frequency noise to choose a palette region and higher-frequency noise to modulate bark detail.

```wgsl
let regional = fbm(world_position.xz * stick.regional_scale, stick.seed);
let detail = fbm(world_position.xyz * stick.detail_scale, stick.seed + 101u);
```

The regional sample drives broad color variation. The detail sample adds local bark irregularity.

