//---------------------------------------------------------
// Stylized explosion lobes: object-space noise displacement,
// age-driven fire heat or warm/cool smoke bands, silhouette dissolve.
//---------------------------------------------------------

#import bevy_pbr::{
    mesh_functions,
    forward_io::{Vertex, VertexOutput},
    view_transformations::position_world_to_clip,
    mesh_view_bindings::{view, globals},
}
#import bevy_core_pipeline::tonemapping::tone_mapping

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> params: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var<uniform> tint: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var<uniform> extras: vec4<f32>;

const KIND_SMOKE: f32 = 1.0;
const KIND_FLASH: f32 = 2.0;

fn hash21(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    let d = dot(p3, p3.yzx + vec3<f32>(33.33));
    return fract((p3.x + p3.y) * p3.z + d);
}

fn hash31(p: vec3<f32>) -> f32 {
    return hash21(p.xy + vec2<f32>(p.z * 1.37, p.z * 0.61));
}

fn value_noise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let n000 = hash31(i);
    let n100 = hash31(i + vec3<f32>(1.0, 0.0, 0.0));
    let n010 = hash31(i + vec3<f32>(0.0, 1.0, 0.0));
    let n110 = hash31(i + vec3<f32>(1.0, 1.0, 0.0));
    let n001 = hash31(i + vec3<f32>(0.0, 0.0, 1.0));
    let n101 = hash31(i + vec3<f32>(1.0, 0.0, 1.0));
    let n011 = hash31(i + vec3<f32>(0.0, 1.0, 1.0));
    let n111 = hash31(i + vec3<f32>(1.0, 1.0, 1.0));
    let x00 = mix(n000, n100, u.x);
    let x10 = mix(n010, n110, u.x);
    let x01 = mix(n001, n101, u.x);
    let x11 = mix(n011, n111, u.x);
    return mix(mix(x00, x10, u.y), mix(x01, x11, u.y), u.z);
}

fn fbm(p: vec3<f32>) -> f32 {
    var n = 0.0;
    var a = 0.5;
    var q = p;
    for (var i = 0; i < 4; i++) {
        n += value_noise(q) * a;
        q = q * 2.03 + vec3<f32>(11.2, 3.7, 7.1);
        a *= 0.5;
    }
    return n;
}

fn soft_band(v: f32, steps: f32) -> f32 {
    let s = max(steps, 2.0);
    let x = saturate(v) * s;
    let q = floor(x);
    let f = fract(x);
    return (q + smoothstep(0.32, 0.68, f)) / s;
}

fn object_pos(vertex: Vertex) -> vec3<f32> {
#ifdef VERTEX_COLORS
    return vertex.color.xyz;
#else
    return vertex.position;
#endif
}

@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;
    var vertex = vertex_no_morph;
    let world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);
    let age_n = saturate(params.x / max(params.y, 1e-3));
    let seed = params.z;
    let local0 = object_pos(vertex);
    let nse = fbm(local0 * 3.4 + vec3<f32>(seed, seed * 0.37, seed * 1.13));
    let displace = extras.z * (nse - 0.48) * (0.55 + 0.45 * age_n);
    var local = vertex.position + vertex.normal * displace;
#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex_no_morph.instance_index,
    );
#endif
#ifdef VERTEX_POSITIONS
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(local, 1.0),
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
    out.color = vec4<f32>(local0, 1.0);
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

fn shade_fire(local: vec3<f32>, n: vec3<f32>, view_dir: vec3<f32>, age_n: f32) -> vec4<f32> {
    let key = normalize(vec3<f32>(0.28, 0.86, 0.32));
    let wrap = saturate(dot(n, key) * 0.55 + 0.45);
    let facing = saturate(abs(dot(n, view_dir)));
    let heat = saturate((1.0 - age_n) * (0.42 + 0.58 * wrap));
    let hot = vec3<f32>(1.12, 0.88, 0.38);
    let mid = vec3<f32>(0.98, 0.34, 0.06);
    let cool = vec3<f32>(0.32, 0.05, 0.02);
    var rgb = mix(cool, mix(mid, hot, saturate(heat * 1.35 - 0.2)), soft_band(heat, extras.w));
    rgb *= tint.xyz;
    let nse = fbm(local * 2.6 + vec3<f32>(params.z, 2.1, 0.4));
    let rim = saturate(1.0 - facing);
    rgb += mid * rim * 0.12 * (1.0 - age_n);
    let body = mix(0.55, 1.0, facing) * extras.x;
    let edge = saturate(1.2 - length(local) * 2.15);
    let dissolve = smoothstep(age_n * 1.12 - 0.12, age_n * 1.12 + 0.22, nse * 0.6 + edge * 0.4);
    let alpha = dissolve * (1.0 - age_n * age_n) * mix(0.5, 0.92, edge);
    return vec4<f32>(rgb * body, alpha);
}

fn shade_smoke(local: vec3<f32>, n: vec3<f32>, view_dir: vec3<f32>, age_n: f32) -> vec4<f32> {
    let key = normalize(vec3<f32>(0.32, 0.84, 0.22));
    let wrap = saturate(dot(n, key) * 0.5 + 0.5);
    let facing = saturate(abs(dot(n, view_dir)));
    let warm = vec3<f32>(0.46, 0.36, 0.28);
    let cool = vec3<f32>(0.18, 0.22, 0.28);
    let lift = soft_band(wrap * (0.75 + 0.25 * facing), extras.w);
    var rgb = mix(cool, warm, lift) * tint.xyz;
    let nse = fbm(local * 2.2 + vec3<f32>(params.z * 0.7, 4.0, 1.2));
    rgb *= 0.78 + 0.22 * nse;
    let edge = saturate(1.15 - length(local) * 2.05);
    let dissolve = smoothstep(age_n * 1.05 - 0.08, age_n * 1.05 + 0.28, nse * 0.55 + edge * 0.45);
    let alpha = dissolve * (1.0 - pow(age_n, 1.35)) * mix(0.42, 0.88, edge);
    return vec4<f32>(rgb * extras.x, alpha);
}

fn shade_flash(local: vec3<f32>, n: vec3<f32>, view_dir: vec3<f32>, age_n: f32) -> vec4<f32> {
    let facing = saturate(abs(dot(n, view_dir)));
    let hot = vec3<f32>(1.4, 1.15, 0.72);
    let limb = vec3<f32>(1.0, 0.55, 0.16);
    let rgb = mix(limb, hot, facing) * tint.xyz;
    let edge = saturate(1.3 - length(local) * 2.4);
    let fade = 1.0 - smoothstep(0.15, 1.0, age_n);
    let alpha = edge * fade * mix(0.35, 1.0, facing);
    return vec4<f32>(rgb * extras.x * fade, alpha);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let view_dir = normalize(view.world_position.xyz - mesh.world_position.xyz);
    let n = normalize(mesh.world_normal);
#ifdef VERTEX_COLORS
    let local = mesh.color.xyz;
#else
    let local = vec3<f32>(0.0);
#endif
    let age_n = saturate(params.x / max(params.y, 1e-3));
    var shaded: vec4<f32>;
    if params.w > 1.5 {
        shaded = shade_flash(local, n, view_dir, age_n);
    } else if params.w > 0.5 {
        shaded = shade_smoke(local, n, view_dir, age_n);
    } else {
        shaded = shade_fire(local, n, view_dir, age_n);
    }
    let mapped = tone_mapping(vec4<f32>(shaded.xyz, 1.0), view.color_grading);
    return vec4<f32>(mapped.rgb, shaded.w);
}
