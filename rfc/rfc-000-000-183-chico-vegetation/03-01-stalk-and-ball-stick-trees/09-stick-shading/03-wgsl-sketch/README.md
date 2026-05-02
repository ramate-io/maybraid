# 3.1.9.3: WGSL Sketch

This page is subsection **3.1.9.3** of [RFC-183: Chico Vegetation](../../../README.md)


```wgsl
struct StickShaderParams {
    seed: u32,
    regional_scale: f32,
    detail_scale: f32,
    value_strength: f32,
    color0: vec3<f32>,
    _pad0: f32,
    color1: vec3<f32>,
    _pad1: f32,
    color2: vec3<f32>,
    _pad2: f32,
    color3: vec3<f32>,
    _pad3: f32,
};

@group(1) @binding(0)
var<uniform> stick: StickShaderParams;

fn fbm(p: vec3<f32>, seed: u32) -> f32 {
    // Use standard value noise, Perlin, or the existing project fbm.
    return fract(sin(dot(p, vec3<f32>(12.9898, 78.233, 37.719)) + f32(seed)) * 43758.5453);
}

fn palette_sample(t: f32) -> vec3<f32> {
    let t = clamp(t, 0.0, 1.0);
    let x = t * 3.0;
    let i = u32(floor(x));
    let f = fract(x);

    if (i == 0u) {
        return mix(stick.color0, stick.color1, f);
    }
    if (i == 1u) {
        return mix(stick.color1, stick.color2, f);
    }

    return mix(stick.color2, stick.color3, f);
}

fn stick_color(world_position: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let regional = fbm(world_position * stick.regional_scale, stick.seed);
    let detail = fbm(world_position * stick.detail_scale, stick.seed + 101u);

    let base = palette_sample(regional);

    let value = mix(
        1.0 - stick.value_strength,
        1.0 + stick.value_strength,
        detail,
    );

    // Optional: darken upward-facing creases less than side-facing bark.
    let side = 1.0 - abs(normal.y);
    let side_shade = mix(0.9, 1.05, side);

    return base * value * side_shade;
}
```

