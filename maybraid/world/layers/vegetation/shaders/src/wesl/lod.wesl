//! Projected-size helpers and nested density selection.
//!
//! Authoring thresholds and buffer budgets stay on the CPU. These functions only
//! evaluate size / fade math at the *actual* internal render resolution.
//!
//! Do not treat subpixel *width* as a reason to drop skyline *height*.

import vegetation_shaders::wesl::layout::GrassViewUniform;

struct LodThresholds {
    near_m: f32,
    mid_m: f32,
    card_m: f32,
    near_density: f32,
    mid_density: f32,
    card_density: f32,
    min_height_px: f32,
}

fn projected_height_px(
    height_m: f32,
    distance_m: f32,
    view_height_px: f32,
    fov_y_radians: f32,
) -> f32 {
    let dist = max(distance_m, 1e-3);
    let denom = 2.0 * tan(fov_y_radians * 0.5);
    return view_height_px * (height_m / dist) / max(denom, 1e-5);
}

fn lod_fade(distance_m: f32, start_m: f32, end_m: f32) -> f32 {
    return saturate((end_m - distance_m) / max(end_m - start_m, 1e-3));
}

fn nested_keep(rank: f32, density: f32) -> bool {
    return rank < saturate(density);
}

/// `0` near ribbon, `1` mid ribbon, `2` card, `3` culled.
fn lod_bucket(
    distance_m: f32,
    height_m: f32,
    rank: f32,
    view_height_px: f32,
    fov_y_radians: f32,
    thresholds: LodThresholds,
) -> u32 {
    let px = projected_height_px(height_m, distance_m, view_height_px, fov_y_radians);
    if (px < thresholds.min_height_px && distance_m > thresholds.card_m) {
        return 3u;
    }
    if (distance_m < thresholds.near_m && nested_keep(rank, thresholds.near_density)) {
        return 0u;
    }
    if (distance_m < thresholds.mid_m && nested_keep(rank, thresholds.mid_density)) {
        return 1u;
    }
    if (distance_m < thresholds.card_m && nested_keep(rank, thresholds.card_density)) {
        return 2u;
    }
    return 3u;
}

/// Catalog selection uses `@workgroup_size(64, 1, 1)`. Extra workgroups tile
/// onto Y when `ceil(count / 64)` would exceed the 65535-per-axis device limit.
fn catalog_select_index(gid: vec3<u32>, num_workgroups: vec3<u32>) -> u32 {
    return gid.x + gid.y * num_workgroups.x * 64u;
}

fn sphere_in_frustum(center: vec3<f32>, radius: f32, clip: mat4x4<f32>) -> bool {
    let p = clip * vec4<f32>(center, 1.0);
    let slack = radius * length(clip[0].xyz) + 0.05;
    let w = p.w + slack;
    return w > 0.0 && abs(p.x) <= w && abs(p.y) <= w && p.z >= -slack && p.z <= w;
}

fn grove_sphere_center(aabb_min: vec3<f32>, aabb_max: vec3<f32>) -> vec3<f32> {
    return (aabb_min + aabb_max) * 0.5;
}

fn grove_sphere_radius(aabb_min: vec3<f32>, aabb_max: vec3<f32>) -> f32 {
    return length(aabb_max - aabb_min) * 0.5;
}

/// `3` means the grove is dropped (inactive, frustum, or past `card_m`).
fn grove_lod_bucket(
    aabb_min: vec3<f32>,
    aabb_max: vec3<f32>,
    flags: u32,
    view: GrassViewUniform,
) -> u32 {
    if ((flags & 1u) == 0u) {
        return 3u;
    }
    let center = grove_sphere_center(aabb_min, aabb_max);
    let radius = grove_sphere_radius(aabb_min, aabb_max);
    if (!sphere_in_frustum(center, radius, view.clip_from_world)) {
        return 3u;
    }
    let thresholds = LodThresholds(
        view.lod_distances.y,
        view.lod_distances.z,
        view.lod_distances.w,
        view.lod_density.x,
        view.lod_density.y,
        view.lod_density.z,
        view.lod_density.w,
    );
    return lod_bucket(
        length(view.world_position.xyz - center),
        max(aabb_max.y - aabb_min.y, 1.0),
        0.0,
        view.resolution.y,
        view.resolution.z,
        thresholds,
    );
}

const MAX_COMPUTE_WORKGROUPS: u32 = 65535u;

fn tiled_dispatch_groups(item_count: u32) -> vec3<u32> {
    let groups = max((item_count + 63u) / 64u, 1u);
    if (groups <= MAX_COMPUTE_WORKGROUPS) {
        return vec3<u32>(groups, 1u, 1u);
    }
    let y = max((groups + MAX_COMPUTE_WORKGROUPS - 1u) / MAX_COMPUTE_WORKGROUPS, 1u);
    var x = (groups + y - 1u) / y;
    if (x > MAX_COMPUTE_WORKGROUPS) {
        x = MAX_COMPUTE_WORKGROUPS;
    }
    return vec3<u32>(max(x, 1u), y, 1u);
}
