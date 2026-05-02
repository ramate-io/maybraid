# 3.1.9.1: Species Palette

This page is subsection **3.1.9.1** of [RFC-183: Chico Vegetation](../../../README.md)


Each tree species provides a small ordered palette of stick colors.

```rust
pub struct StickPalette {
    pub colors: [Vec3; 4],
    pub regional_scale: f32,
    pub detail_scale: f32,
    pub value_strength: f32,
}
```

Example palette:

```rust
let bark_palette = [
    vec3(0.22, 0.15, 0.10), // dark brown
    vec3(0.36, 0.26, 0.18), // warm bark
    vec3(0.42, 0.36, 0.28), // gray bark
    vec3(0.16, 0.12, 0.09), // dark crevice
];
```

Species can bias toward gray, red, yellow, black, or pale bark tones.

