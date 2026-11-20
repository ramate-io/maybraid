//! Compact bush GPU structs. Keep in lockstep with `layout.rs`.

struct PlantDescriptor {
    root: vec3<f32>,
    height_m: f32,
    up: vec3<f32>,
    plant_id: u32,
    aabb_min: vec3<f32>,
    branch_offset: u32,
    aabb_max: vec3<f32>,
    branch_count: u32,
    leaf_offset: u32,
    leaf_count: u32,
    seed: u32,
    flags: u32,
    crown_radius_m: f32,
    wood_stiffness: f32,
    foliage_density: f32,
    species: f32,
}

struct BranchDescriptor {
    p0: vec3<f32>,
    parent_index: u32,
    p1: vec3<f32>,
    parent_t: f32,
    p2: vec3<f32>,
    plant_index: u32,
    root_radius_m: f32,
    tip_radius_m: f32,
    stiffness: f32,
    keep_rank: f32,
    twist: f32,
    depth: u32,
    branch_id: u32,
    major_parent: u32,
}

struct LeafDescriptor {
    plant_index: u32,
    parent_branch: u32,
    attach_t: f32,
    size_m: f32,
    twist: f32,
    fold: f32,
    flutter: f32,
    density_rank: f32,
    facing: vec3<f32>,
    leaf_id: u32,
    palette: vec3<f32>,
    shape: f32,
}
