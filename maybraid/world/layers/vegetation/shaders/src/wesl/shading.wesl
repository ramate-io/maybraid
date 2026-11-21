//! Inexpensive two-sided vegetation shading. Palette and light are explicit.
//!
//! Painterly BotW-like: root darkening, wrap lighting, controlled backlight.
//! No photoreal noise. Opaque by default — callers decide alpha.

import vegetation_shaders::wesl::hash::{hash_u32, hash_u32_to_unit, value_noise_2};

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

/// Inner/lower shells stay cooler and darker; outer/top shells go warm and light.
/// `shell` is `0` inner → `1` outer. Compose with `grass_variation_palette` for hue.
fn crown_layer_albedo(
    palette: VegetationPalette,
    height_frac: f32,
    radial_frac: f32,
    shell: f32,
    variation: f32,
) -> vec3<f32> {
    let cool = palette.root_rgb * vec3<f32>(0.52, 0.78, 1.38);
    let warm = palette.tip_rgb * vec3<f32>(1.28, 1.10, 0.68);
    let along = saturate(height_frac * 0.42 + saturate(radial_frac) * 0.28 + saturate(shell) * 0.38);
    let mixed = mix(cool, warm, smoothstep(0.10, 0.88, along));
    return mixed * (0.84 + 0.28 * saturate(variation));
}

/// Billowy canopy clump: height warmth plus a cooler, darker underside.
fn canopy_clump_albedo(
    warm_rgb: vec3<f32>,
    cool_rgb: vec3<f32>,
    height_frac: f32,
    normal: vec3<f32>,
    clump_variation: f32,
) -> vec3<f32> {
    let sunlit = crown_albedo(warm_rgb, cool_rgb, height_frac, clump_variation);
    let lift = smoothstep(-0.22, 0.48, normalize(normal).y);
    return sunlit * mix(vec3<f32>(0.28, 0.40, 0.58), vec3<f32>(1.0, 1.0, 1.0), lift);
}

/// Ambient-occlusion-like darkening for the crown core and downward faces.
fn canopy_interior_shade(
    height_frac: f32,
    radial_frac: f32,
    shell: f32,
    normal: vec3<f32>,
) -> f32 {
    let n = normalize(normal);
    let under = 1.0 - smoothstep(-0.12, 0.40, n.y);
    let core = (1.0 - saturate(shell)) * (1.0 - saturate(radial_frac) * 0.52);
    let low = 1.0 - saturate(height_frac);
    return mix(0.26, 1.0, 1.0 - saturate(under * 0.58 + core * 0.64 + low * 0.20));
}

fn hashed_leaf_blob(p: vec2<f32>, cell: vec2<i32>, seed: u32) -> f32 {
    let packed = (u32(cell.x + 173) * 374761393u) ^ (u32(cell.y + 91) * 668265263u);
    let h0 = hash_u32(seed ^ hash_u32(packed));
    let h1 = hash_u32(h0 ^ 0x85ebca6bu);
    let ox = hash_u32_to_unit(h1);
    let oy = hash_u32_to_unit(h1 ^ 0xc2b2ae35u);
    let stretch = 0.70 + 0.24 * hash_u32_to_unit(h1 ^ 0x27d4eb2du);
    let center = vec2<f32>(f32(cell.x) + ox * 0.74, f32(cell.y) + oy * 0.74);
    let delta = p - center;
    let v = saturate(delta.y * 0.55 + 0.5);
    let petiole = smoothstep(0.0, 0.22, v);
    let tip = 1.0 - 0.55 * smoothstep(0.48, 1.0, v);
    let width = max(petiole * tip, 0.16);
    let q = vec2<f32>(delta.x / (0.38 * stretch * width), delta.y / (0.52 * stretch));
    return 1.0 - smoothstep(0.58, 1.0, length(q));
}

/// Hashed small-leaf occupancy inside a card. `uv` is `(u in [-1,1], v in [0,1])`.
/// `detail` 1 = near shaggy cutout, 0 = far solid clump (no holes).
fn leaf_cluster_mask(uv: vec2<f32>, seed: u32, detail: f32) -> f32 {
    let d = saturate(detail);
    let cells_v = mix(2.2, 6.4, d);
    let cells_u = mix(1.8, 5.2, d);
    let p = vec2<f32>((uv.x * 0.5 + 0.5) * cells_u, saturate(uv.y) * cells_v);
    let base = vec2<i32>(i32(floor(p.x)), i32(floor(p.y)));
    var occ = 0.0;
    occ = max(occ, hashed_leaf_blob(p, base + vec2<i32>(-1, -1), seed));
    occ = max(occ, hashed_leaf_blob(p, base + vec2<i32>(0, -1), seed));
    occ = max(occ, hashed_leaf_blob(p, base + vec2<i32>(1, -1), seed));
    occ = max(occ, hashed_leaf_blob(p, base + vec2<i32>(-1, 0), seed));
    occ = max(occ, hashed_leaf_blob(p, base + vec2<i32>(0, 0), seed));
    occ = max(occ, hashed_leaf_blob(p, base + vec2<i32>(1, 0), seed));
    occ = max(occ, hashed_leaf_blob(p, base + vec2<i32>(-1, 1), seed));
    occ = max(occ, hashed_leaf_blob(p, base + vec2<i32>(0, 1), seed));
    occ = max(occ, hashed_leaf_blob(p, base + vec2<i32>(1, 1), seed));
    let filled = saturate(occ * mix(1.55, 1.12, d));
    return mix(1.0, filled, d);
}

/// Per-mini-leaf hue wobble. Cheap; not photoreal noise.
fn leaf_cluster_tint(uv: vec2<f32>, seed: u32) -> f32 {
    let cell = vec2<i32>(i32(floor((uv.x * 0.5 + 0.5) * 5.0)), i32(floor(saturate(uv.y) * 6.0)));
    let packed = (u32(cell.x + 19) * 374761393u) ^ (u32(cell.y + 7) * 668265263u);
    return hash_u32_to_unit((seed ^ hash_u32(packed)) ^ 0xa0761d65u);
}

/// Dithered keep/discard for when MSAA / alpha-to-coverage is off. `true` = keep.
fn leaf_cluster_keep(alpha: f32, pixel: vec2<f32>, seed: u32, detail: f32) -> bool {
    if (detail < 0.08) {
        return true;
    }
    if (alpha < 0.06) {
        return false;
    }
    if (alpha > 0.92) {
        return true;
    }
    let hx = hash_u32((seed ^ (u32(pixel.x) * 1973u)) ^ (u32(pixel.y) * 9277u));
    return alpha > hash_u32_to_unit(hx) * 0.78 + 0.08;
}

/// Bend the card normal toward the clump outward so overlapping shells read as a mass.
fn leaf_cluster_outward_normal(
    geometric: vec3<f32>,
    clump_out: vec3<f32>,
    uv: vec2<f32>,
    detail: f32,
) -> vec3<f32> {
    let toward = normalize(clump_out + vec3<f32>(0.0, 0.12, 0.0));
    let edge = saturate(abs(uv.x));
    let amt = saturate(detail) * mix(0.48, 0.18, edge);
    return normalize(mix(normalize(geometric), toward, amt));
}
