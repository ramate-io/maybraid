//! Stable integer hashing for plant / primitive identity and nested density ranks.
//!
//! Units: hashes are dimensionless `u32`. `density_rank` is in `[0, 1)`.
//! Identity must not depend on compaction append order.

fn hash_u32(value: u32) -> u32 {
    var state = value;
    state = state ^ 2747636419u;
    state = state * 2654435769u;
    state = state ^ (state >> 16u);
    state = state * 2654435769u;
    state = state ^ (state >> 16u);
    state = state * 2654435769u;
    return state;
}

fn hash_u32_to_unit(value: u32) -> f32 {
    return f32(hash_u32(value)) * (1.0 / 4294967295.0);
}

/// Stable plant identity from a patch seed and a local index inside that patch.
fn plant_id(patch_seed: u32, local_index: u32) -> u32 {
    return hash_u32(patch_seed ^ hash_u32(local_index ^ 0x9e3779b9u));
}

/// Nested density rank in `[0, 1)`. Keep a plant at a LOD when `rank < threshold`.
fn density_rank(id: u32) -> f32 {
    return hash_u32_to_unit(id ^ 0x85ebca6bu);
}

fn hash_lattice(cell: vec2<i32>) -> f32 {
    let x = u32(cell.x) * 374761393u;
    let y = u32(cell.y) * 668265263u;
    return hash_u32_to_unit(x ^ y ^ 0x9e3779b9u);
}

/// Smooth value noise in `[0, 1)` for low-frequency meadow colour patches.
fn value_noise_2(p: vec2<f32>) -> f32 {
    let cell = vec2<i32>(floor(p));
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash_lattice(cell);
    let b = hash_lattice(cell + vec2<i32>(1, 0));
    let c = hash_lattice(cell + vec2<i32>(0, 1));
    let d = hash_lattice(cell + vec2<i32>(1, 1));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}
