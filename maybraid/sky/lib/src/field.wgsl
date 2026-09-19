//---------------------------------------------------------
// Cosmos backdrop. Opaque Cosimo field behind the blue dome.
// Day does not crush this — the dome's alpha does.
//
// Sun / moon live on the dome. This pass is nebula + hash stars.
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
    let f = f0 * f0 * (3.0 - 2.0 * f0);
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
    return value_noise3(p) * 0.65 + value_noise3(p * 2.13) * 0.35;
}

fn poster(n: f32, steps: f32) -> f32 {
    return floor(saturate(n) * steps + 0.5) / steps;
}

fn flatten(n: f32) -> f32 {
    return mix(n, poster(n, 6.0), 0.45);
}

fn celestial_dots(dir: vec3<f32>, t: f32) -> vec3<f32> {
    let crawl = vec3<f32>(t * 0.027, -t * 0.021, t * 0.07);
    let g = dir * 72.0 + crawl;
    let cell = floor(g);
    let f = fract(g) - 0.5;
    let h = hash13(cell);
    let h2 = hash13(cell + vec3<f32>(3.1, 1.4, 7.2));
    let star = step(0.994, h) * (1.0 - smoothstep(0.06, 0.16, length(f)));
    let pg = dir * 11.0 + crawl * 0.35;
    let pcell = floor(pg);
    let pf = fract(pg) - 0.5;
    let p = hash13(pcell);
    let p2 = hash13(pcell + vec3<f32>(8.0, 0.2, 4.0));
    let planet = step(0.988, p) * step(0.55, p2) * (1.0 - smoothstep(0.10, 0.22, length(pf)));
    var rgb = mix(vec3<f32>(0.96, 0.92, 1.0), vec3<f32>(0.72, 0.94, 1.0), step(0.5, h2)) * star;
    rgb += mix(vec3<f32>(1.0, 0.68, 0.36), vec3<f32>(0.60, 0.78, 1.0), step(0.5, p2)) * planet * 0.9;
    return rgb;
}

fn shade_cosmos(dir: vec3<f32>) -> vec3<f32> {
    let t = globals.time;
    let swirl_g = material.style.y;
    let star_g = material.style.z;
    let phase = material.style.w;
    let crawl = vec3<f32>(t * 0.026, -t * 0.022, 0.0) + vec3<f32>(phase * 0.35, 0.0, 0.0);
    let morph = t * 0.08;

    let n = flatten(fbm3(dir * 3.4 + crawl + vec3<f32>(0.0, 0.0, morph)));
    let n2 = flatten(value_noise3(dir * 8.2 + crawl * 0.6 + vec3<f32>(-morph * 0.5, morph * 0.4, 0.4)));

    let day = material.style.x;
    let void_c = mix(vec3<f32>(0.012, 0.004, 0.04), vec3<f32>(0.05, 0.02, 0.12), day);
    let nebula = mix(vec3<f32>(0.07, 0.015, 0.16), vec3<f32>(0.28, 0.06, 0.48), day);
    let bloom = mix(vec3<f32>(0.16, 0.04, 0.28), vec3<f32>(0.52, 0.16, 0.72), day);
    var color = mix(void_c, nebula, n);
    color = mix(color, bloom, smoothstep(0.50, 0.78, n2) * 0.38 * swirl_g);

    let band = smoothstep(0.72, 0.88, 0.5 + 0.5 * sin(dir.x * 2.2 + dir.y * 3.1 + t * 0.35));
    color += mix(vec3<f32>(0.16, 0.05, 0.26), vec3<f32>(0.62, 0.22, 0.78), day) * band * 0.12 * swirl_g;
    color += celestial_dots(dir, t) * star_g;
    return color;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(in.world_position.xyz - view.world_position.xyz);
    return vec4<f32>(shade_cosmos(dir), 1.0);
}
