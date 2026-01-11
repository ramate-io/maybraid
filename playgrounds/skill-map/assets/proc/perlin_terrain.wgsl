#define_import_path proc:perlin_terrain;

// ============================================================================
// Terrain SDF + Perlin Noise Helper Module (NO BINDINGS)
// This file is safe to @import anywhere.
// ============================================================================

// ----------------------------------------------------------------------------
// Configuration structs (passed from caller/pre-imported file)
// ----------------------------------------------------------------------------
struct TerrainConfig {
    height_scale : f32,
    _padding     : vec3<f32>,
};

struct Bounds {
    enabled : u32,       // 0 or 1
    min     : vec2<f32>, // bounding rectangle min
    max     : vec2<f32>, // bounding rectangle max
};

// ----------------------------------------------------------------------------
// Hashing + gradient noise (Perlin-style)
// ----------------------------------------------------------------------------
fn hash2(p: vec2<i32>, seed: i32) -> f32 {
    // Integer hash → [0, 1)
    var x : i32 = p.x * 374761393 +
                  p.y * 668265263 +
                  seed * 951274213;

    x = (x ^ (x >> 13)) * 1274126177;
    x = x ^ (x >> 16);

    // Convert to float in 0..1
    return f32(x & 0x7fffffff) / f32(0x7fffffff);
}

fn grad2(hash: f32, f: vec2<f32>) -> f32 {
    let angle = hash * 6.28318530718; // 2π
    let g = vec2<f32>(cos(angle), sin(angle));
    return dot(g, f);
}

fn perlin2(p: vec2<f32>, seed: i32) -> f32 {
    let p0 = floor(p);
    let pf = p - p0;
    let ip = vec2<i32>(i32(p0.x), i32(p0.y));

    let corners = array<vec2<i32>, 4>(
        ip + vec2<i32>(0, 0),
        ip + vec2<i32>(1, 0),
        ip + vec2<i32>(0, 1),
        ip + vec2<i32>(1, 1)
    );

    let f = pf;
    let u = f * f * (3.0 - 2.0 * f);

    let n00 = grad2(hash2(corners[0], seed), f - vec2<f32>(0.0, 0.0));
    let n10 = grad2(hash2(corners[1], seed), f - vec2<f32>(1.0, 0.0));
    let n01 = grad2(hash2(corners[2], seed), f - vec2<f32>(0.0, 1.0));
    let n11 = grad2(hash2(corners[3], seed), f - vec2<f32>(1.0, 1.0));

    let nx0 = mix(n00, n10, u.x);
    let nx1 = mix(n01, n11, u.x);

    return mix(nx0, nx1, u.y); // approx [-1, 1]
}

// ----------------------------------------------------------------------------
// Heightfield logic
// ----------------------------------------------------------------------------
fn height_at(
    world_x : f32,
    world_z : f32,
    config  : TerrainConfig,
    bounds  : Bounds,
    seed    : i32
) -> f32 {
    // Bounds check
    if (bounds.enabled == 1u) {
        if (world_x < bounds.min.x ||
            world_x > bounds.max.x ||
            world_z < bounds.min.y ||
            world_z > bounds.max.y) {
            return 0.0;
        }
    }

    var height     : f32 = 0.0;
    var amplitude  : f32 = 1.0;
    var frequency  : f32 = 0.05;

    for (var i: i32 = 0; i < 4; i = i + 1) {
        let sample = perlin2(
            vec2<f32>(world_x * frequency, world_z * frequency),
            seed
        );
        height = height + sample * amplitude;

        amplitude = amplitude * 0.5;
        frequency = frequency * 2.0;
    }

    // Contrast shaping
    let exponent : f32 = 1.1;
    let sign_val = select(-1.0, 1.0, height >= 0.0);
    let h = sign_val * pow(abs(height), exponent);

    return h * config.height_scale;
}

// ----------------------------------------------------------------------------
// Terrain SDF (top surface + bedrock clamp)
// ----------------------------------------------------------------------------
fn terrain_sdf(
    p      : vec3<f32>,
    config : TerrainConfig,
    bounds : Bounds,
    seed   : i32
) -> f32 {
    var terrain_height = height_at(p.x, p.z, config, bounds, seed);

    // Ridge/plateau shaping
    if (terrain_height > 10.0) {
        terrain_height = 10.0 + 0.75 * (terrain_height - 10.0);
    } else if (terrain_height < -10.0) {
        terrain_height = -10.0 - 0.75 * (terrain_height + 10.0);
    }

    let bedrock_level = -config.height_scale * 4.0;

    let d_surface = p.y - terrain_height;
    let d_bedrock = bedrock_level - p.y;

    return max(d_surface, d_bedrock);
}

// Public wrapper
fn sdf(
    p      : vec3<f32>,
    config : TerrainConfig,
    bounds : Bounds,
    seed   : i32
) -> f32 {
    return terrain_sdf(p, config, bounds, seed);
}
