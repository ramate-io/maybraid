//! Compact GPU buffer layouts. Keep in lockstep with `layout.rs`.
//!
//! Units: world metres, seconds, unitless ranks in `[0, 1)`. Alignment is std430
//! for storage and std140 for uniforms.

struct BladeDescriptor {
    /// World-space root (metres).
    root: vec3<f32>,
    /// Blade length along the rest curve (metres).
    height_m: f32,
    /// Surface normal at the root (unit).
    normal: vec3<f32>,
    /// Rest width at the root (metres).
    width_m: f32,
    /// Preferred facing / bend direction (unit).
    facing: vec3<f32>,
    /// Stable plant identity. Independent of compaction order.
    plant_id: u32,
    /// x = bend, y = taper, z = stiffness, w = density_rank.
    shape: vec4<f32>,
    /// xyz = linear RGB variation seed, w = species id as f32.
    palette: vec4<f32>,
}

struct GrassPatchHeader {
    aabb_min: vec3<f32>,
    blade_offset: u32,
    aabb_max: vec3<f32>,
    blade_count: u32,
    seed: u32,
    coverage: f32,
    species: u32,
    flags: u32,
}

struct GrassViewUniform {
    clip_from_world: mat4x4<f32>,
    world_from_clip: mat4x4<f32>,
    /// xyz = camera world position (metres), w = time (seconds).
    world_position: vec4<f32>,
    /// xyz = previous camera position, w = previous time (seconds).
    prev_world_position: vec4<f32>,
    /// xy = actual render resolution (pixels), z = fov Y (radians), w = view kind.
    resolution: vec4<f32>,
    /// xyz = wind direction XZ + unused, w = wind strength (metres).
    wind: vec4<f32>,
    /// x = speed, y = near_m, z = mid_m, w = card_m.
    lod_distances: vec4<f32>,
    /// x = near density, y = mid, z = card, w = min height px.
    lod_density: vec4<f32>,
    /// x = blade count, y = capacity per LOD list, z = light x, w = light y (z in extras).
    counts: vec4<f32>,
    light_direction: vec4<f32>,
}

struct GrassPartingUniform {
    camera_clip_from_world: mat4x4<f32>,
    camera_world_from_clip: mat4x4<f32>,
    /// x = strength (m), y = radius (m), z = depth falloff (m), w = mode.
    params: vec4<f32>,
    /// x = lod skip (m), y = ground reject (m), z = kernel (px), w = occluder count.
    extras: vec4<f32>,
    camera_right: vec4<f32>,
    camera_forward: vec4<f32>,
    /// xyz = game-camera world position (metres). Shared by shadow draws.
    camera_position: vec4<f32>,
    /// xyz = world centre (m), w = radius (m).
    occluders: array<vec4<f32>, 16>,
    /// x = height (m), y = active, zw unused.
    occluder_meta: array<vec4<f32>, 16>,
}

struct GrassDrawParams {
    /// 0 near, 1 mid, 2 card.
    lod_bucket: u32,
    index_count: u32,
    vertex_count: u32,
    view_kind: u32,
}

struct DrawIndexedIndirect {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
}
