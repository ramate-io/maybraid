#import proc::marching_cubes 
#import proc::perlin_terrain::TerrainConfig
#import proc::perlin_terrain::Bounds

// ============================================================================
// Mesh Generation Pass — positions, normals (via SDF gradient), UVs
// ============================================================================

struct Sampling3D {
    chunk_origin : vec3<f32>,   // world offset (chunk_origin)
    chunk_size   : vec3<f32>,   // chunk size in world units
    resolution   : vec3<u32>,   // voxel resolution (nx, ny, nz)
};

@group(0) @binding(0)
var<uniform> sampling : Sampling3D;

@group(0) @binding(1)
var<storage, read> cube_index : array<u32>;

@group(0) @binding(2)
var<storage, read> tri_offset : array<u32>; // prefix-summed *triangle* counts

@group(0) @binding(3)
var<storage, read_write> out_positions : array<vec3<f32>>;

@group(0) @binding(4)
var<storage, read_write> out_normals   : array<vec3<f32>>;

@group(0) @binding(5)
var<storage, read_write> out_uvs       : array<vec2<f32>>;

@group(0) @binding(6)
var<uniform> terrain_config : TerrainConfig;

@group(0) @binding(7)
var<uniform> bounds : Bounds;

@group(0) @binding(8)
var<uniform> seed : i32;


// ----------------------------------------------------------------------------
// Compute finite difference gradient of the SDF for smooth normals
// ----------------------------------------------------------------------------
fn sdf_normal(p : vec3<f32>, config : TerrainConfig, bounds : Bounds, seed   : i32) -> vec3<f32> {
    // Scale epsilon to voxel size so normals stay stable across LOD
    let voxel = sampling.chunk_size / vec3<f32>(sampling.resolution);
    let eps   = min(voxel.x, min(voxel.y, voxel.z)) * 0.5;

    let dx = sdf(vec3<f32>(p.x + eps, p.y,        p.z       ), terrain_config, bounds, seed) -
             sdf(vec3<f32>(p.x - eps, p.y,        p.z       ), terrain_config, bounds, seed);

    let dy = sdf(vec3<f32>(p.x,        p.y + eps, p.z       ), terrain_config, bounds, seed) -
             sdf(vec3<f32>(p.x,        p.y - eps, p.z       ), terrain_config, bounds, seed);

    let dz = sdf(vec3<f32>(p.x,        p.y,        p.z + eps), terrain_config, bounds, seed) -
             sdf(vec3<f32>(p.x,        p.y,        p.z - eps), terrain_config, bounds, seed);

    return normalize(vec3<f32>(dx, dy, dz));
}


// ----------------------------------------------------------------------------
// Main mesh generation
// ----------------------------------------------------------------------------
@compute @workgroup_size(8, 8, 8)
fn compute_mesh(@builtin(global_invocation_id) gid : vec3<u32>) {

    // Guard against the last layer (no cell beyond max-1)
    if (gid.x >= sampling.resolution.x - 1u ||
        gid.y >= sampling.resolution.y - 1u ||
        gid.z >= sampling.resolution.z - 1u)
    {
        return;
    }

    let nx = sampling.resolution.x;
    let ny = sampling.resolution.y;
    let flat = gid.x + gid.y * nx + gid.z * nx * ny;

    let cube = cube_index[flat];
    if (cube == 0u || cube == 255u) {
        return; // completely empty or full
    }

    // tri_offset is the number of triangles emitted by all previous voxels
    let tri_base = tri_offset[flat];

    let grid_f    = vec3<f32>(f32(gid.x), f32(gid.y), f32(gid.z));
    let cell_size = sampling.chunk_size / vec3<f32>(sampling.resolution);
    let p         = sampling.chunk_origin + grid_f * cell_size;

    // Corner SDF reuse
    let corner = array<f32, 8>(
        sdf(p + cell_size * vec3<f32>(0.0, 0.0, 0.0), terrain_config, bounds, seed),
        sdf(p + cell_size * vec3<f32>(1.0, 0.0, 0.0), terrain_config, bounds, seed),
        sdf(p + cell_size * vec3<f32>(1.0, 1.0, 0.0), terrain_config, bounds, seed),
        sdf(p + cell_size * vec3<f32>(0.0, 1.0, 0.0), terrain_config, bounds, seed),

        sdf(p + cell_size * vec3<f32>(0.0, 0.0, 1.0), terrain_config, bounds, seed),
        sdf(p + cell_size * vec3<f32>(1.0, 0.0, 1.0), terrain_config, bounds, seed),
        sdf(p + cell_size * vec3<f32>(1.0, 1.0, 1.0), terrain_config, bounds, seed),
        sdf(p + cell_size * vec3<f32>(0.0, 1.0, 1.0), terrain_config, bounds, seed)
    );

    var ti : u32 = 0u;
    loop {
        let e0 = marching_triangles(cube, ti);
        if (e0 < 0) {
            break;
        }

        let e1 = marching_triangles(cube, ti + 1u);
        let e2 = marching_triangles(cube, ti + 2u);

        let v0 = interpolate_mc_vertex(u32(e0), p, cell_size, corner);
        let v1 = interpolate_mc_vertex(u32(e1), p, cell_size, corner);
        let v2 = interpolate_mc_vertex(u32(e2), p, cell_size, corner);

        // Global triangle index for this cell + this local triangle
        let tri_index = tri_base + (ti / 3u);
        let base      = tri_index * 3u; // 3 verts per triangle

        // Positions
        out_positions[base + 0u] = v0;
        out_positions[base + 1u] = v1;
        out_positions[base + 2u] = v2;

        // Smooth normals via SDF gradient (in WORLD space)
        out_normals[base + 0u] = sdf_normal(v0, terrain_config, bounds, seed);
        out_normals[base + 1u] = sdf_normal(v1, terrain_config, bounds, seed);
        out_normals[base + 2u] = sdf_normal(v2, terrain_config, bounds, seed);

        // UVs: tile over the chunk in local XZ
        let local0 = v0 - sampling.chunk_origin;
        let local1 = v1 - sampling.chunk_origin;
        let local2 = v2 - sampling.chunk_origin;

        let size_x = sampling.chunk_size.x;
        let size_z = sampling.chunk_size.z;

        out_uvs[base + 0u] = vec2<f32>(local0.x / size_x, local0.z / size_z);
        out_uvs[base + 1u] = vec2<f32>(local1.x / size_x, local1.z / size_z);
        out_uvs[base + 2u] = vec2<f32>(local2.x / size_x, local2.z / size_z);

        ti += 3u;
    }
}
