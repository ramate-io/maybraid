//! View-dependent vegetation parting / swathing.
//!
//! Roots stay pinned. Only the upper parameter `t` deflects, with the same
//! `t² * (1 - stiffness)` envelope as wind so rest-arch + wind + parting compose
//! without double-applying lean. Call this *after* `deform_along_curve`.
//!
//! Coordinate space: world metres. Depth values are reverse-Z clip depths in
//! `[0, 1]` (1 = near). Bushes and canopies should import these functions rather
//! than copy them; texture fetches stay in the entry-point shader.

import vegetation_shaders::wesl::hash::hash_u32_to_unit;

const PARTING_MODE_OFF: u32 = 0u;
const PARTING_MODE_DEPTH: u32 = 1u;
const PARTING_MODE_RASTER: u32 = 2u;
const PARTING_MODE_WORLD: u32 = 3u;

fn parting_along(t: f32, stiffness: f32) -> f32 {
    let along = saturate(t);
    return along * along * saturate(1.0 - stiffness);
}

fn max_parting_displacement_m(height_m: f32, strength_m: f32, stiffness: f32) -> f32 {
    return height_m * strength_m * saturate(1.0 - stiffness);
}

/// `1` when the occluder sits just behind the blade, `0` if it is in front,
/// coplanar (self-parting), or beyond `falloff_m`.
fn depth_gap_weight(blade_dist_m: f32, occluder_dist_m: f32, falloff_m: f32) -> f32 {
    let gap = occluder_dist_m - blade_dist_m;
    if (gap <= 1e-3) {
        return 0.0;
    }
    let falloff = max(falloff_m, 1e-3);
    let enter = smoothstep(0.0, 0.08, gap);
    let leave = 1.0 - smoothstep(falloff * 0.35, falloff, gap);
    return saturate(enter * leave);
}

/// Reject the ground / turf under a blade. Occluders must stand above `root_y`.
fn ground_occluder_weight(occluder_y: f32, root_y: f32, ground_reject_m: f32) -> f32 {
    return smoothstep(root_y + ground_reject_m * 0.35, root_y + ground_reject_m, occluder_y);
}

fn clip_to_uv(clip: vec4<f32>) -> vec2<f32> {
    let w = max(abs(clip.w), 1e-5);
    let ndc = clip.xy / w;
    return ndc * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5);
}

fn uv_to_ndc(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    return vec3<f32>(uv.x * 2.0 - 1.0, -(uv.y * 2.0 - 1.0), depth);
}

fn reconstruct_world(uv: vec2<f32>, depth: f32, world_from_clip: mat4x4<f32>) -> vec3<f32> {
    let clip = vec4<f32>(uv_to_ndc(uv, depth), 1.0);
    let world = world_from_clip * clip;
    return world.xyz / max(world.w, 1e-5);
}

fn screen_part_direction(gradient: vec2<f32>, plant_id: u32) -> vec2<f32> {
    let len = length(gradient);
    if (len > 0.045) {
        return -gradient / len;
    }
    let side = select(-1.0, 1.0, hash_u32_to_unit(plant_id ^ 0xc2b2ae35u) >= 0.5);
    return vec2<f32>(side, 0.0);
}

/// Flatten a screen-space parting direction into world XZ, perpendicular to view.
fn world_part_from_screen(
    dir_ss: vec2<f32>,
    camera_right: vec3<f32>,
    camera_up: vec3<f32>,
    camera_forward: vec3<f32>,
) -> vec3<f32> {
    var world = camera_right * dir_ss.x + camera_up * dir_ss.y;
    let fwd = camera_forward;
    world = world - fwd * dot(world, fwd);
    world.y = 0.0;
    let len = length(world);
    if (len < 1e-5) {
        var fallback = camera_right;
        fallback.y = 0.0;
        let fl = length(fallback);
        if (fl < 1e-5) {
            return vec3<f32>(1.0, 0.0, 0.0);
        }
        return fallback / fl;
    }
    return world / len;
}

fn capsule_behind_weight(
    root: vec3<f32>,
    camera: vec3<f32>,
    occluder_center: vec3<f32>,
    camera_forward: vec3<f32>,
    falloff_m: f32,
) -> f32 {
    let fwd = camera_forward;
    let blade_along = dot(root - camera, fwd);
    let occ_along = dot(occluder_center - camera, fwd);
    return depth_gap_weight(blade_along, occ_along, falloff_m);
}

fn capsule_lateral_xz(
    root_xz: vec2<f32>,
    occluder_xz: vec2<f32>,
    forward_xz: vec2<f32>,
) -> vec2<f32> {
    var fwd = forward_xz;
    if (length(fwd) < 1e-4) {
        fwd = vec2<f32>(0.0, 1.0);
    }
    fwd = normalize(fwd);
    let to_root = root_xz - occluder_xz;
    return to_root - fwd * dot(to_root, fwd);
}

fn capsule_away_xz(
    root_xz: vec2<f32>,
    occluder_xz: vec2<f32>,
    camera_right_xz: vec2<f32>,
    forward_xz: vec2<f32>,
) -> vec2<f32> {
    var away = capsule_lateral_xz(root_xz, occluder_xz, forward_xz);
    let dist = length(away);
    if (dist < 1e-4) {
        var right = camera_right_xz;
        if (length(right) < 1e-4) {
            right = vec2<f32>(1.0, 0.0);
        }
        return normalize(right);
    }
    return away / dist;
}

fn capsule_radial_weight(
    root_xz: vec2<f32>,
    occluder_xz: vec2<f32>,
    forward_xz: vec2<f32>,
    radius_m: f32,
) -> f32 {
    let dist = length(capsule_lateral_xz(root_xz, occluder_xz, forward_xz));
    let influence = max(radius_m, 1e-3);
    return 1.0 - smoothstep(influence * 0.35, influence, dist);
}

/// Keep the longer of two pre-envelope XZ offsets so depth and world can
/// compose without adding (which would double-apply tagged occluders).
fn stronger_part_xz(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return select(b, a, dot(a, a) >= dot(b, b));
}

fn part_along_curve(
    rest: vec3<f32>,
    root: vec3<f32>,
    t: f32,
    part_xz: vec2<f32>,
    stiffness: f32,
) -> vec3<f32> {
    let along = saturate(t);
    let weight = parting_along(along, stiffness);
    let offset = vec3<f32>(part_xz.x * weight, -length(part_xz) * weight * 0.06, part_xz.y * weight);
    var parted = rest + offset;
    if (along <= 1e-4) {
        parted = root;
    }
    return parted;
}
