//---------------------------------------------------------
// Blue dome: 4D blue / haze patches with alpha holes.
// Cosmos sits behind this shell.
//---------------------------------------------------------

#import bevy_pbr::{
    mesh_functions,
    forward_io::{Vertex, VertexOutput},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::{view, globals},
}

struct SkyDomeParams {
    zenith: vec4<f32>,
    horizon: vec4<f32>,
    nadir: vec4<f32>,
    // x day_weight, y peak_alpha, z unused, w phase
    style: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: SkyDomeParams;

@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;
    var vertex = vertex_no_morph;
    let world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);
#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex_no_morph.instance_index,
    );
#endif
#ifdef VERTEX_POSITIONS
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0),
    );
    out.position = position_world_to_clip(out.world_position.xyz);
#endif
#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
        vertex_no_morph.instance_index,
    );
#endif
#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex_no_morph.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_no_morph.instance_index,
        world_from_local[3],
    );
#endif
    return out;
}

fn hash12(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * vec3<f32>(0.1031, 0.1030, 0.0973));
    let d = p3 + vec3<f32>(dot(p3, p3.yzx + 33.33));
    return fract((d.x + d.y) * d.z);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f0 = fract(p);
    let f = f0 * f0 * f0 * (f0 * (f0 * 6.0 - 15.0) + 10.0);
    let a = hash12(i);
    let b = hash12(i + vec2<f32>(1.0, 0.0));
    let c = hash12(i + vec2<f32>(0.0, 1.0));
    let d = hash12(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

fn fbm(p: vec2<f32>) -> f32 {
    return value_noise(p) * 0.5
        + value_noise(p * 1.87) * 0.28
        + value_noise(p * 3.41) * 0.15
        + value_noise(p * 6.13) * 0.07;
}

fn sky_uv(dir: vec3<f32>) -> vec2<f32> {
    return vec2<f32>(atan2(dir.z, dir.x), dir.y);
}

/// How much of the sky is atmosphere right now. Three slow clocks, not a sine.
fn haze_cover(t: f32) -> f32 {
    let a = fbm(vec2<f32>(t * 0.0065, 2.17));
    let b = fbm(vec2<f32>(-t * 0.0042, 8.41));
    let c = fbm(vec2<f32>(t * 0.0026 + 5.2, -t * 0.0031));
    return mix(0.22, 0.84, saturate(a * 0.50 + b * 0.32 + c * 0.18));
}

/// Large drifting islands: high = haze, low = blue.
fn haze_island(dir: vec3<f32>, t: f32) -> f32 {
    let p = sky_uv(dir) * 1.15;
    let drift = vec2<f32>(t * 0.016, -t * 0.012);
    let warp = fbm(p + drift);
    let n = fbm(p * 1.2 + vec2<f32>(warp * 1.15, t * 0.009));
    let n2 = fbm(p * 2.05 + vec2<f32>(-t * 0.014, warp));
    return saturate(n * 0.62 + n2 * 0.38);
}

/// Separate 4D field for cosmos holes so blue and haze can both open up.
fn cosmos_hole(dir: vec3<f32>, t: f32) -> f32 {
    let p = sky_uv(dir) * 1.05 + vec2<f32>(3.7, -1.4);
    let n = fbm(p + vec2<f32>(t * 0.011, -t * 0.008));
    let n2 = fbm(p * 1.7 + vec2<f32>(-t * 0.007, t * 0.013));
    return saturate(n * 0.58 + n2 * 0.42);
}

fn shade_dome(dir: vec3<f32>) -> vec4<f32> {
    let elev = dir.y;
    let t = globals.time;
    let day = material.style.x;
    let peak = material.style.y;

    var haze_col = material.horizon.xyz;
    if elev < 0.0 {
        haze_col = mix(material.horizon.xyz, material.nadir.xyz, saturate(-elev));
    }
    let blue = material.zenith.xyz;
    let island = haze_island(dir, t);
    let haze = smoothstep(0.32, 0.68, island);
    let rgb = mix(blue, haze_col, haze);

    let cover = haze_cover(t);
    let hole = cosmos_hole(dir, t);
    // Day thickens the dome; cover wanders how much of it is solid.
    let presence = mix(0.10, 0.86, day) * mix(0.50, 1.0, cover);
    // Holes open more at night; afternoon still punches islands of cosmos.
    let open = smoothstep(mix(0.28, 0.48, day), mix(0.62, 0.86, day), hole);
    var alpha = presence * (1.0 - open * mix(0.92, 0.62, day));
    // Horizon keeps a band so ridges still wash, even through a hole.
    let rim = smoothstep(0.38, -0.06, elev);
    alpha = max(alpha, rim * mix(0.22, 0.55, day));
    alpha = saturate(alpha * peak);

    return vec4<f32>(rgb, alpha);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(in.world_position.xyz - view.world_position.xyz);
    return shade_dome(dir);
}
