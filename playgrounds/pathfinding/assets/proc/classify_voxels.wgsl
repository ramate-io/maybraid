#import proc::marching_cubes
#import proc::perlin_terrain::TerrainConfig
#import proc::perlin_terrain::Bounds

// ============================================================================
// Classification Pass — computes cubeIndex + triangleCount per voxel
// ============================================================================

struct Sampling3D {
    chunk_origin : vec3<f32>,
    chunk_size   : vec3<f32>,
    resolution   : vec3<u32>,
};

@group(0) @binding(0)
var<uniform> sampling : Sampling3D;

@group(0) @binding(1)
var<storage, read_write> cube_index : array<u32>;

@group(0) @binding(2)
var<storage, read_write> tri_counts : array<u32>;

// Optional seed/bounds/config imported by perlin_terrain
@group(0) @binding(3)
var<uniform> terrain_config : TerrainConfig;

@group(0) @binding(4)
var<uniform> bounds : Bounds;

@group(0) @binding(5)
var<uniform> seed : i32;


// ----------------------------------------------------------------------------
// Compute cube index + triangle count
// ----------------------------------------------------------------------------
@compute @workgroup_size(8, 8, 8)
fn classify(@builtin(global_invocation_id) gid : vec3<u32>) {

    if (gid.x >= sampling.resolution.x - 1u ||
        gid.y >= sampling.resolution.y - 1u ||
        gid.z >= sampling.resolution.z - 1u)
    {
        return;
    }

    let grid_f = vec3<f32>(f32(gid.x), f32(gid.y), f32(gid.z));
    let cell_size = sampling.chunk_size / vec3<f32>(sampling.resolution);

    let p = sampling.chunk_origin + grid_f * cell_size;

    // Evaluate eight SDF corners
    let d000 = sdf(p + cell_size * vec3<f32>(0.0, 0.0, 0.0), terrain_config, bounds, seed);
    let d100 = sdf(p + cell_size * vec3<f32>(1.0, 0.0, 0.0), terrain_config, bounds, seed);
    let d010 = sdf(p + cell_size * vec3<f32>(0.0, 1.0, 0.0), terrain_config, bounds, seed);
    let d110 = sdf(p + cell_size * vec3<f32>(1.0, 1.0, 0.0), terrain_config, bounds, seed);

    let d001 = sdf(p + cell_size * vec3<f32>(0.0, 0.0, 1.0), terrain_config, bounds, seed);
    let d101 = sdf(p + cell_size * vec3<f32>(1.0, 0.0, 1.0), terrain_config, bounds, seed);
    let d011 = sdf(p + cell_size * vec3<f32>(0.0, 1.0, 1.0), terrain_config, bounds, seed);
    let d111 = sdf(p + cell_size * vec3<f32>(1.0, 1.0, 1.0), terrain_config, bounds, seed);

    // Cube index mask
    var cube : u32 = 0u;
    if (d000 < 0.0) { cube |= 1u; }
    if (d100 < 0.0) { cube |= 2u; }
    if (d010 < 0.0) { cube |= 4u; }
    if (d110 < 0.0) { cube |= 8u; }
    if (d001 < 0.0) { cube |= 16u; }
    if (d101 < 0.0) { cube |= 32u; }
    if (d011 < 0.0) { cube |= 64u; }
    if (d111 < 0.0) { cube |= 128u; }

    let nx = sampling.resolution.x;
    let ny = sampling.resolution.y;
    let flat = gid.x + gid.y * nx + gid.z * nx * ny;

    cube_index[flat] = cube;

    // Number of triangles for this cube
    tri_counts[flat] = marching_cube_triangle_count(cube);
}
