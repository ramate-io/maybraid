//! Projected-size helpers and nested density selection.
//!
//! Authoring thresholds and buffer budgets stay on the CPU. These functions only
//! evaluate size / fade math at the *actual* internal render resolution.
//!
//! Do not treat subpixel *width* as a reason to drop skyline *height*.

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
