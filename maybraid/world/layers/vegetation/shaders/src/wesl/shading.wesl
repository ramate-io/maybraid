//! Inexpensive two-sided vegetation shading. Palette and light are explicit.
//!
//! Painterly BotW-like: root darkening, wrap lighting, controlled backlight.
//! No photoreal noise. Opaque by default — callers decide alpha.

import vegetation_shaders::wesl::hash::{hash_u32_to_unit, value_noise_2};

struct VegetationPalette {
    root_rgb: vec3<f32>,
    tip_rgb: vec3<f32>,
}

/// Low-frequency meadow colour field. Patches drift across tens of metres.
fn meadow_color_field(world_xz: vec2<f32>) -> f32 {
    let n0 = value_noise_2(world_xz * 0.085);
    let n1 = value_noise_2(world_xz * 0.031 + vec2<f32>(17.2, 9.4));
    return saturate(n0 * 0.62 + n1 * 0.38);
}

/// Per-blade / per-patch palette: cooler dark roots, warm light tips, yellow-green
/// and deep-green patches, occasional dry gold. `species_mid` tints the mix.
fn grass_variation_palette(
    species_mid: vec3<f32>,
    world_xz: vec2<f32>,
    plant_id: u32,
) -> VegetationPalette {
    let field = meadow_color_field(world_xz);
    let u = hash_u32_to_unit(plant_id ^ 0x7feb352du);
    let v = hash_u32_to_unit(plant_id ^ 0x27d4eb2du);
    let deep = vec3<f32>(0.05, 0.22, 0.08);
    let lush = vec3<f32>(0.12, 0.40, 0.10);
    let yellow = vec3<f32>(0.38, 0.52, 0.10);
    let gold = vec3<f32>(0.52, 0.44, 0.12);
    let yellow_amt = saturate(field * 0.82 + u * 0.38);
    var mid = mix(deep, mix(lush, yellow, yellow_amt), 0.38 + 0.55 * field);
    mid = mix(mid, species_mid, 0.16);
    if (v > 0.93) {
        mid = mix(mid, gold, 0.78);
    }
    let root_rgb = mid * vec3<f32>(0.32, 0.48, 0.62);
    let tip_rgb = mid * vec3<f32>(1.38, 1.22, 0.78) + vec3<f32>(0.06, 0.10, 0.02);
    return VegetationPalette(root_rgb, saturate(tip_rgb));
}

fn grass_albedo(palette: VegetationPalette, t: f32, variation: f32) -> vec3<f32> {
    let tt = saturate(t);
    let mid = mix(palette.root_rgb, palette.tip_rgb, 0.38);
    let along = mix(
        mix(palette.root_rgb, mid, saturate(tt * 1.45)),
        palette.tip_rgb,
        smoothstep(0.38, 1.0, tt),
    );
    return along * (0.88 + 0.20 * saturate(variation));
}

fn two_sided_wrap(normal: vec3<f32>, light_dir: vec3<f32>, wrap: f32) -> f32 {
    let n = normalize(normal);
    let l = normalize(light_dir);
    return saturate(abs(dot(n, l)) * wrap + (1.0 - wrap));
}

fn painterly_lit(
    albedo: vec3<f32>,
    normal: vec3<f32>,
    light_dir: vec3<f32>,
    t: f32,
) -> vec3<f32> {
    let wrap = two_sided_wrap(normal, light_dir, 0.38);
    let n = normalize(normal);
    let l = normalize(light_dir);
    let back = saturate(-dot(n, l)) * 0.16;
    let sky = saturate(n.y * 0.40 + 0.66) * 0.30;
    let band0 = mix(0.48, 1.0, smoothstep(0.10, 0.40, wrap));
    let band1 = mix(band0, 1.08, smoothstep(0.55, 0.92, wrap));
    let root_dark = mix(0.52, 1.0, smoothstep(0.0, 0.38, saturate(t)));
    return albedo * (band1 * root_dark + back + sky);
}

fn wood_albedo(palette: VegetationPalette, t: f32, variation: f32) -> vec3<f32> {
    let along = mix(palette.root_rgb, palette.tip_rgb, saturate(t * 0.55 + 0.12));
    return along * (0.78 + 0.28 * saturate(variation));
}

/// Soft wrap + stronger backlight so leaves read translucent without a scatter model.
fn leaf_painterly_lit(
    albedo: vec3<f32>,
    normal: vec3<f32>,
    light_dir: vec3<f32>,
    t: f32,
) -> vec3<f32> {
    let wrap = two_sided_wrap(normal, light_dir, 0.62);
    let n = normalize(normal);
    let l = normalize(light_dir);
    let back = saturate(-dot(n, l)) * 0.34;
    let sky = saturate(n.y * 0.55 + 0.50) * 0.34;
    let band0 = mix(0.38, 1.05, smoothstep(0.10, 0.32, wrap));
    let band1 = mix(band0, 1.18, smoothstep(0.55, 0.82, wrap));
    let tip_lift = mix(0.78, 1.0, smoothstep(0.0, 0.45, saturate(t)));
    return albedo * (band1 * tip_lift + back + sky);
}

/// Warm sunlit rim → cool interior across a crown (`height_frac` in `0..=1`).
fn crown_albedo(warm_rgb: vec3<f32>, cool_rgb: vec3<f32>, height_frac: f32, variation: f32) -> vec3<f32> {
    let h = saturate(height_frac);
    let along = mix(cool_rgb, warm_rgb, smoothstep(0.18, 0.88, h));
    return along * (0.88 + 0.22 * saturate(variation));
}
