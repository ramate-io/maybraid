//! Inexpensive two-sided vegetation shading. Palette and light are explicit.
//!
//! Painterly BotW-like: root darkening, wrap lighting, controlled backlight.
//! No photoreal noise. Opaque by default — callers decide alpha.

struct VegetationPalette {
    root_rgb: vec3<f32>,
    tip_rgb: vec3<f32>,
}

fn grass_albedo(palette: VegetationPalette, t: f32, variation: f32) -> vec3<f32> {
    let tt = saturate(t);
    let mid = mix(palette.root_rgb, palette.tip_rgb, 0.42);
    let along = mix(
        mix(palette.root_rgb, mid, saturate(tt * 1.55)),
        palette.tip_rgb,
        smoothstep(0.42, 1.0, tt),
    );
    return along * (0.90 + 0.18 * saturate(variation));
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
    let wrap = two_sided_wrap(normal, light_dir, 0.42);
    let n = normalize(normal);
    let l = normalize(light_dir);
    let back = saturate(-dot(n, l)) * 0.18;
    let sky = saturate(n.y * 0.45 + 0.62) * 0.28;
    let band0 = mix(0.42, 1.0, smoothstep(0.14, 0.28, wrap));
    let band1 = mix(band0, 1.12, smoothstep(0.58, 0.78, wrap));
    let root_dark = mix(0.58, 1.0, smoothstep(0.0, 0.34, saturate(t)));
    return albedo * (band1 * root_dark + back + sky);
}
