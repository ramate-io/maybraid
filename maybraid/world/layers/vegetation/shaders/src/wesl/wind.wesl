//! Root-anchored wind field and curve deformation.
//!
//! Inputs are explicit so later branch-attached foliage can share the same field
//! without double-applying gusts: sample `wind_field` once, then `deform_along_curve`
//! with a per-primitive stiffness (`0` = grass, higher = wood).
//!
//! Coordinate space: world metres. `root_xz` is the plant's anchored XZ. Time is
//! seconds. Wind direction is a unit XZ vector.

struct WindInputs {
    direction_xz: vec2<f32>,
    speed: f32,
    strength_m: f32,
    time_s: f32,
}

fn wind_field(root_xz: vec2<f32>, inputs: WindInputs) -> vec2<f32> {
    var dir = inputs.direction_xz;
    if (length(dir) < 1e-5) {
        dir = vec2<f32>(0.92, 0.38);
    }
    dir = normalize(dir);
    let travelling = dot(root_xz, vec2<f32>(0.058, 0.034)) - inputs.time_s * inputs.speed;
    let gust = 0.72 * sin(travelling) + 0.28 * sin(travelling * 2.3 - inputs.time_s * 1.7);
    let wave = sin(dot(root_xz, dir) * 0.08 - inputs.time_s * 0.55);
    return dir * (inputs.strength_m * (0.65 * gust + 0.35 * wave));
}

fn max_wind_displacement_m(height_m: f32, strength_m: f32, stiffness: f32) -> f32 {
    return height_m * strength_m * saturate(1.0 - stiffness);
}

fn deform_along_curve(
    rest: vec3<f32>,
    root: vec3<f32>,
    t: f32,
    wind_xz: vec2<f32>,
    stiffness: f32,
) -> vec3<f32> {
    let along = saturate(t);
    let weight = along * along * saturate(1.0 - stiffness);
    let offset = vec3<f32>(
        wind_xz.x * weight,
        -length(wind_xz) * weight * 0.12,
        wind_xz.y * weight,
    );
    var deformed = rest + offset;
    // Keep the root pinned even if `t` is slightly off due to quantization.
    if (along <= 1e-4) {
        deformed = root;
    }
    return deformed;
}
