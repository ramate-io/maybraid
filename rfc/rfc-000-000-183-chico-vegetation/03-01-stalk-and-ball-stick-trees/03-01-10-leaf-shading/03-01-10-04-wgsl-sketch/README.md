# 3.1.10.4: WGSL Sketch

This page is subsection **3.1.10.4** of [RFC-183: Chico Vegetation](../../../README.md)


```wgsl
const MAX_LEAF_COLORS: u32 = 4u;
const MAX_FLECKS: u32 = 4u;

struct LeafFleck {
    color: vec3<f32>,
    strength: f32,

    season_center: f32,
    season_width: f32,
    season_cutoff: f32,
    longitude_divisor: f32,

    altitude_start: f32,
    altitude_end: f32,
    altitude_divisor: f32,
    noise_scale: f32,

    season_weight: f32,
    longitude_weight: f32,
    altitude_weight: f32,
    noise_cutoff: f32,
};

struct LeafShaderParams {
    seed: u32,
    color_count: u32,
    fleck_count: u32,
    _pad0: u32,

    regional_scale: f32,
    detail_scale: f32,
    value_strength: f32,
    _pad1: f32,

    colors: array<vec4<f32>, 4>,
    flecks: array<LeafFleck, 4>,
};

@group(1) @binding(0)
var<uniform> leaf: LeafShaderParams;

@group(1) @binding(1)
var<uniform> season_time: f32;

fn fbm(p: vec3<f32>, seed: u32) -> f32 {
    // Placeholder: use standard value noise, Perlin, or project fbm.
    return fract(sin(dot(p, vec3<f32>(12.9898, 78.233, 37.719)) + f32(seed)) * 43758.5453);
}

fn cyclic_window(t: f32, center: f32, width: f32) -> f32 {
    let d = abs(fract(t - center + 0.5) - 0.5);
    return smoothstep(width, 0.0, d);
}

fn palette_sample(t: f32) -> vec3<f32> {
    let count = max(leaf.color_count, 1u);
    let max_i = count - 1u;

    let x = clamp(t, 0.0, 1.0) * f32(max_i);
    let i = min(u32(floor(x)), max_i);
    let j = min(i + 1u, max_i);
    let f = fract(x);

    return mix(
        leaf.colors[i].rgb,
        leaf.colors[j].rgb,
        f,
    );
}

fn fleck_mask(
    fleck: LeafFleck,
    world_position: vec3<f32>,
    seed: u32,
) -> f32 {
    let season_term = cyclic_window(
        season_time,
        fleck.season_center,
        fleck.season_width,
    );

    if (season_term < fleck.season_cutoff) {
        return 0.0;
    }

    let lon_term = fbm(
        vec3<f32>(
            world_position.x / max(fleck.longitude_divisor, 0.0001),
            0.0,
            0.0,
        ),
        seed + 17u,
    );

    let alt_base = smoothstep(
        fleck.altitude_start,
        fleck.altitude_end,
        world_position.y,
    );

    let alt_noise = fbm(
        vec3<f32>(
            0.0,
            world_position.y / max(fleck.altitude_divisor, 0.0001),
            0.0,
        ),
        seed + 31u,
    );

    let altitude_term = alt_base * alt_noise;

    let denom = max(
        0.0001,
        fleck.season_weight
            + fleck.longitude_weight
            + fleck.altitude_weight,
    );

    let env = (
        season_term * fleck.season_weight
        + lon_term * fleck.longitude_weight
        + altitude_term * fleck.altitude_weight
    ) / denom;

    let local = fbm(world_position * fleck.noise_scale, seed + 47u);
    let mask = env * local;

    if (mask < fleck.noise_cutoff) {
        return 0.0;
    }

    return mask;
}

fn leaf_color(world_position: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let regional = fbm(world_position * leaf.regional_scale, leaf.seed);
    let detail = fbm(world_position * leaf.detail_scale, leaf.seed + 101u);

    var color = palette_sample(regional);

    let value = mix(
        1.0 - leaf.value_strength,
        1.0 + leaf.value_strength,
        detail,
    );

    color = color * value;

    for (var i = 0u; i < MAX_FLECKS; i = i + 1u) {
        if (i >= leaf.fleck_count) {
            break;
        }

        let fleck = leaf.flecks[i];
        let mask = fleck_mask(fleck, world_position, leaf.seed + i * 131u);
        let amount = clamp(mask * fleck.strength, 0.0, 1.0);

        color = mix(color, fleck.color, amount);
    }

    return color;
}
```

