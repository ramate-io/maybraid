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

/// Linear taper, metres. Used by woody tubes; grass keeps `blade_width_at`.
fn branch_radius_at(root_radius_m: f32, tip_radius_m: f32, t: f32) -> f32 {
    return max(mix(root_radius_m, tip_radius_m, saturate(t)), 1e-4);
}

/// Quadratic Bézier rest curve. Control points are metres in the curve's local space
/// (world for a trunk, parent attachment frame for a child).
fn branch_rest_sample(p0: vec3<f32>, p1: vec3<f32>, p2: vec3<f32>, t: f32) -> CurveSample {
    let tt = saturate(t);
    let u = 1.0 - tt;
    let position = u * u * p0 + 2.0 * u * tt * p1 + tt * tt * p2;
    var tangent = 2.0 * u * (p1 - p0) + 2.0 * tt * (p2 - p1);
    if (length(tangent) < 1e-5) {
        tangent = p2 - p0;
    }
    let frame = stable_frame(tangent, vec3<f32>(0.0, 1.0, 0.0));
    return CurveSample(position, frame.along, frame);
}

/// Radial `0..=1` around the tube. Collar overlap is authored by starting `t` slightly
/// negative in the parent frame, not by a second mesh.
fn woody_surface_vertex(sample: CurveSample, radius_m: f32, radial: f32) -> vec3<f32> {
    let angle = radial * 6.28318530718;
    let dir = sample.frame.across * cos(angle) + sample.frame.facing * sin(angle);
    return sample.position + dir * radius_m;
}

fn woody_surface_normal(sample: CurveSample, radial: f32) -> vec3<f32> {
    let angle = radial * 6.28318530718;
    return normalize(sample.frame.across * cos(angle) + sample.frame.facing * sin(angle));
}

/// Folded leaf in an attachment frame. `u` is across `[-1, 1]`, `v` is base→tip `[0, 1]`.
fn leaf_surface_vertex(
    origin: vec3<f32>,
    frame: LocalFrame,
    size_m: f32,
    u: f32,
    v: f32,
    fold: f32,
) -> vec3<f32> {
    let vv = saturate(v);
    let across = frame.across * (u * size_m * 0.46 * (1.0 - vv * 0.38));
    let along = frame.along * (vv * size_m);
    let cup = frame.facing * (fold * u * u * size_m * 0.16 + fold * vv * (1.0 - vv) * size_m * 0.22);
    return origin + across + along + cup;
}

fn leaf_surface_normal(frame: LocalFrame, u: f32, fold: f32) -> vec3<f32> {
    return normalize(frame.facing + frame.across * (u * 0.18) + frame.along * (fold * 0.12) + vec3<f32>(0.0, 0.35, 0.0));
}
