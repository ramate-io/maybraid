//! Whole-grove culling before per-item selection.

struct WoodyGroveRecord {
    aabb_min: vec3<f32>,
    branch_off: u32,
    aabb_max: vec3<f32>,
    branch_count: u32,
    plant_off: u32,
    plant_count: u32,
    leaf_off: u32,
    leaf_count: u32,
    seed: u32,
    flags: u32,
}

struct GroveVisibleSpan {
    branch_off: u32,
    branch_count: u32,
    leaf_off: u32,
    leaf_count: u32,
}

fn grove_active(record: WoodyGroveRecord) -> bool {
    return (record.flags & 1u) != 0u;
}

fn grove_center(record: WoodyGroveRecord) -> vec3<f32> {
    return (record.aabb_min + record.aabb_max) * 0.5;
}

fn grove_radius(record: WoodyGroveRecord) -> f32 {
    return length(record.aabb_max - record.aabb_min) * 0.5;
}

fn aabb_in_frustum(minv: vec3<f32>, maxv: vec3<f32>, clip: mat4x4<f32>) -> bool {
    let center = (minv + maxv) * 0.5;
    let radius = length(maxv - minv) * 0.5 + 0.5;
    let p = clip * vec4<f32>(center, 1.0);
    let slack = radius * length(clip[0].xyz) + 0.05;
    let w = p.w + slack;
    return w > 0.0 && abs(p.x) <= w && abs(p.y) <= w && p.z >= -slack && p.z <= w;
}
