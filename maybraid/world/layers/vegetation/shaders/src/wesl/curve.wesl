//! Curve evaluation, taper, stable frames, and ribbon surfaces.
//!
//! Coordinate space: world metres. `t` is unitless along the primitive, `0` at the
//! root / attachment and `1` at the tip. Frames are right-handed: X across, Y along
//! the tangent, Z facing.

struct LocalFrame {
    across: vec3<f32>,
    along: vec3<f32>,
    facing: vec3<f32>,
}

struct CurveSample {
    position: vec3<f32>,
    tangent: vec3<f32>,
    frame: LocalFrame,
}

fn stable_frame(tangent: vec3<f32>, up_hint: vec3<f32>) -> LocalFrame {
    var along = tangent;
    let along_len = length(along);
    if (along_len < 1e-5) {
        along = up_hint;
    }
    along = normalize(along);
    var reference = up_hint;
    if (abs(dot(along, reference)) > 0.94) {
        reference = vec3<f32>(1.0, 0.0, 0.0);
        if (abs(dot(along, reference)) > 0.94) {
            reference = vec3<f32>(0.0, 0.0, 1.0);
        }
    }
    let across = normalize(cross(reference, along));
    let facing = cross(across, along);
    return LocalFrame(across, along, facing);
}

/// Parent-child attachment frame. Grass uses this at the root; bushes reuse it at
/// branch parameters `t` so foliage stays welded through wind.
fn attachment_frame(sample: CurveSample, twist_radians: f32) -> LocalFrame {
    let c = cos(twist_radians);
    let s = sin(twist_radians);
    let across = sample.frame.across * c + sample.frame.facing * s;
    let facing = -sample.frame.across * s + sample.frame.facing * c;
    return LocalFrame(normalize(across), sample.frame.along, normalize(facing));
}

fn blade_width_at(width_m: f32, taper: f32, t: f32) -> f32 {
    let tt = saturate(t);
    let profile = pow(1.0 - tt, max(taper, 0.15));
    return max(width_m * profile, 1e-4);
}

fn blade_rest_sample(
    root: vec3<f32>,
    up: vec3<f32>,
    bend_dir: vec3<f32>,
    height_m: f32,
    bend: f32,
    t: f32,
) -> CurveSample {
    let tt = saturate(t);
    let upn = normalize(up);
    var sideways = bend_dir - upn * dot(bend_dir, upn);
    if (length(sideways) < 1e-5) {
        sideways = vec3<f32>(upn.z, 0.0, -upn.x);
        if (length(sideways) < 1e-5) {
            sideways = vec3<f32>(1.0, 0.0, 0.0);
        }
    }
    sideways = normalize(sideways);
    // Roots stay vertical; lean and tip-droop increase with `bend` so some blades arch over.
    let sag_k = 0.16 + 0.58 * saturate(bend * 0.72);
    let rise = tt - sag_k * tt * tt * tt;
    let sweep = tt * tt * (0.42 + 0.58 * tt);
    let position = root + upn * (height_m * rise) + sideways * (bend * height_m * sweep);
    let d_rise = 1.0 - 3.0 * sag_k * tt * tt;
    let d_sweep = 2.0 * 0.42 * tt + 3.0 * 0.58 * tt * tt;
    let tangent = upn * (height_m * d_rise) + sideways * (bend * height_m * d_sweep);
    let frame = stable_frame(tangent, upn);
    return CurveSample(position, frame.along, frame);
}

fn ribbon_vertex(sample: CurveSample, side: f32, half_width_m: f32) -> vec3<f32> {
    return sample.position + sample.frame.across * (side * half_width_m);
}

fn ribbon_normal(sample: CurveSample, side: f32) -> vec3<f32> {
    // Up-biased so orbit/overhead lighting stays painterly, not needle-dark.
    return normalize(
        sample.frame.facing * 0.45 + sample.frame.across * (side * 0.10) + vec3<f32>(0.0, 0.85, 0.0),
    );
}
