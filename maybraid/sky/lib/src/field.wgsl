//---------------------------------------------------------
// Cosmos backdrop. Opaque Cosimo field behind the blue dome.
// Day does not crush this — the dome's alpha does.
//---------------------------------------------------------

#import bevy_pbr::{
    mesh_functions,
    forward_io::{Vertex, VertexOutput},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::{view, globals},
}

struct SkyFieldParams {
    zenith: vec4<f32>,
    horizon: vec4<f32>,
    nadir: vec4<f32>,
    // x day_weight, y swirl_gain, z star_gain, w phase
    style: vec4<f32>,
    sun_dir: vec4<f32>,
    moon_dir: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> material: SkyFieldParams;

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

fn hash13(p: vec3<f32>) -> f32 {
    let q = fract(p * 0.1031);
    let d = q + vec3<f32>(dot(q, q.yzx + 33.33));
    return fract((d.x + d.y) * d.z);
}

fn value_noise3(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f0 = fract(p);
    let f = f0 * f0 * f0 * (f0 * (f0 * 6.0 - 15.0) + 10.0);
    let n000 = hash13(i);
    let n100 = hash13(i + vec3<f32>(1.0, 0.0, 0.0));
    let n010 = hash13(i + vec3<f32>(0.0, 1.0, 0.0));
    let n110 = hash13(i + vec3<f32>(1.0, 1.0, 0.0));
    let n001 = hash13(i + vec3<f32>(0.0, 0.0, 1.0));
    let n101 = hash13(i + vec3<f32>(1.0, 0.0, 1.0));
    let n011 = hash13(i + vec3<f32>(0.0, 1.0, 1.0));
    let n111 = hash13(i + vec3<f32>(1.0, 1.0, 1.0));
    let x00 = mix(n000, n100, f.x);
    let x10 = mix(n010, n110, f.x);
    let x01 = mix(n001, n101, f.x);
    let x11 = mix(n011, n111, f.x);
    return mix(mix(x00, x10, f.y), mix(x01, x11, f.y), f.z);
}

fn fbm3(p: vec3<f32>) -> f32 {
    return value_noise3(p) * 0.50
        + value_noise3(p * 1.87) * 0.28
        + value_noise3(p * 3.41) * 0.15
        + value_noise3(p * 6.13) * 0.07;
}

/// Cosimo `shade_cosmos`, sampled on the view sphere. Slow fbm, sparse pow glints.
fn shade_cosmos(dir: vec3<f32>) -> vec3<f32> {
    let t = globals.time;
    let swirl_g = material.style.y;
    let star_g = material.style.z;
    let phase = material.style.w;
    let drift = vec3<f32>(t * 0.012, -t * 0.01, t * 0.008) + vec3<f32>(phase * 0.35, 0.0, 0.0);

    let n = fbm3(dir * 4.6 + drift);
    let n2 = fbm3(dir * 10.8 + vec3<f32>(-t * 0.016, t * 0.014, 0.4) + drift * 0.6);
    let n3 = fbm3(dir * 6.8 + vec3<f32>(2.1, -t * 0.009, t * 0.011));

    let void_c = vec3<f32>(0.04, 0.02, 0.09);
    let nebula = vec3<f32>(0.22, 0.06, 0.38);
    let bloom = vec3<f32>(0.42, 0.16, 0.62);
    var color = mix(void_c, nebula, n);
    color = mix(color, bloom, smoothstep(0.55, 0.9, n2) * 0.42 * swirl_g);

    let band = 0.5 + 0.5 * sin(dir.x * 2.2 + dir.y * 3.1 + t * 0.35 + n * 2.0);
    let meridian = 0.5 + 0.5 * sin(dir.z * 2.8 - dir.y * 1.6 + t * 0.2);
    color += vec3<f32>(0.55, 0.28, 0.72) * smoothstep(0.75, 1.0, band) * 0.14 * swirl_g;
    color += vec3<f32>(0.35, 0.18, 0.55) * smoothstep(0.88, 1.0, meridian) * 0.08 * swirl_g;

    let glint = pow(saturate(n2), 12.0) * (0.5 + 0.5 * sin(t * 1.4 + n * 6.0));
    let glint2 = pow(saturate(n), 14.0);
    let planet = pow(saturate(n3), 18.0);
    color += vec3<f32>(0.95, 0.82, 1.0) * glint * 0.55 * star_g;
    color += vec3<f32>(0.70, 0.90, 1.0) * glint2 * 0.25 * star_g;
    color += vec3<f32>(0.92, 0.78, 0.55) * planet * 0.40 * star_g;

    let sun = normalize(material.sun_dir.xyz + vec3<f32>(1e-5, 0.0, 0.0));
    let sun_d = saturate(dot(dir, sun));
    let sun_vis = smoothstep(-0.10, 0.04, sun.y);
    color += vec3<f32>(1.0, 0.93, 0.68) * pow(sun_d, 720.0) * 2.4 * sun_vis;
    color += vec3<f32>(1.0, 0.70, 0.32) * pow(sun_d, 28.0) * 0.45 * sun_vis;

    let moon = normalize(material.moon_dir.xyz + vec3<f32>(1e-5, 0.0, 0.0));
    let moon_d = saturate(dot(dir, moon));
    color += vec3<f32>(0.78, 0.84, 0.96) * pow(moon_d, 260.0) * 1.1;
    color += vec3<f32>(0.42, 0.50, 0.70) * pow(moon_d, 16.0) * 0.14;

    return color;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(in.world_position.xyz - view.world_position.xyz);
    return vec4<f32>(shade_cosmos(dir), 1.0);
}
